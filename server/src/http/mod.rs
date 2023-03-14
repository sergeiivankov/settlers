mod api;
mod helpers;
mod serve;
mod ws;

use hyper::{
  body::{ Body, Incoming }, header::{ CACHE_CONTROL, EXPIRES }, server::conn::http1::Builder,
  service::Service as HyperService, Request, StatusCode
};
use lazy_static::initialize;
use log::{ debug, info };
use std::{
  convert::Infallible, future::Future, marker::Unpin, net::SocketAddr, pin::Pin, sync::Arc
};
use tokio::{
  io::{ AsyncRead, AsyncWrite }, net::{ TcpListener, TcpStream },
  sync::{ oneshot::Receiver, Mutex }, task::spawn, select
};
use crate::{ communicator::Communicator, helpers::exit_with_error, settings::SETTINGS };
use self::{
  api::{ ROUTE_HANDLERS, api },
  helpers::{
    MAX_HTTP_BODY_SIZE, HEADER_VALUES, MIME_TYPES, WEB_SOCKET_CONFIG,
    HttpResponse, PreBuiltHeader, header_value, status_response
  },
  serve::serve,
  ws::ws
};

#[cfg(any(feature = "client_resources_caching", feature = "client_resources_packing"))]
use hyper::header::ACCEPT_ENCODING;
#[cfg(any(feature = "client_resources_caching", feature = "client_resources_packing"))]
use self::{ helpers::header_list_contains, serve::CLIENT_RESOURCES };

#[cfg(feature = "secure_server")]
use rustls_pemfile::{ certs, rsa_private_keys };
#[cfg(feature = "secure_server")]
use tokio_rustls::{ rustls::{ Certificate, PrivateKey, ServerConfig }, TlsAcceptor };
#[cfg(feature = "secure_server")]
use std::{ fs::File, io::BufReader };

// MAX_HTTP_BODY_SIZE const will not be more than u32::MAX, so truncation is impossible
#[allow(clippy::cast_possible_truncation)]
const MAX_HTTP_BODY_USIZE: usize = MAX_HTTP_BODY_SIZE as usize;

#[derive(Clone)]
struct Service {
  communicator: Arc<Mutex<Communicator>>
}

impl HyperService<Request<Incoming>> for Service {
  type Response = HttpResponse;
  type Error = Infallible;
  type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send>>;

  fn call(&mut self, req: Request<Incoming>) -> Self::Future {
    let communicator = self.communicator.clone();
    Box::pin(async { Ok(handle_connection(req, communicator).await) })
  }
}

fn build_section_subpath(path: &str) -> (&str, String) {
  // For root "/" path return index.html
  if path.is_empty() {
    return ("public", "index.html".to_string())
  }

  // Split path to section and subpath
  let (section, subpath) = path.split_once('/').unwrap_or((path, ""));

  // For "api" and "ws" sections return as is
  match section {
    "api" | "ws" => return (section, subpath.to_string()),
    _ => {}
  }

  if section == "public" {
    // No return html files for paths like "/public/foo.html"
    if subpath.rsplit_once('.').map_or("", |parts| parts.1) == "html" {
      return ("", String::new())
    }

    return (section, subpath.to_string())
  }

  // For paths like "/foo" return "foo.html", except "index.html"
  if subpath.is_empty() && section != "index" && !section.contains('.') {
    return ("public", format!("{section}.html"))
  }

  ("", String::new())
}

async fn handle_connection(
  req: Request<Incoming>, communicator: Arc<Mutex<Communicator>>
) -> HttpResponse {
  // Main check payload size for all HTTP requests
  // For API requests (except profile picture upload) separate limit
  // For WebSocket messages limit set in stream creation
  let Some(body_size) = req.body().size_hint().upper() else {
    return status_response(StatusCode::LENGTH_REQUIRED)
  };

  if body_size > MAX_HTTP_BODY_SIZE {
    debug!("HTTP body too large: {body_size} > {MAX_HTTP_BODY_SIZE}");
    return status_response(StatusCode::PAYLOAD_TOO_LARGE)
  }

  let uri = req.uri().clone();
  let path = uri.path().get(1..).unwrap_or("");

  let (section, subpath) = build_section_subpath(path);

  match section {
    "public" => {
      #[cfg(any(feature = "client_resources_caching", feature = "client_resources_packing"))]
      {
        let headers = req.headers();
        if !header_list_contains(headers, &ACCEPT_ENCODING, "gzip") {
          return status_response(StatusCode::UNSUPPORTED_MEDIA_TYPE)
        }
      }

      serve(&subpath, req).await
    },
    "api" => {
      let mut response = api(&subpath, req, body_size).await;
      let headers = response.headers_mut();

      // Disable caching for API requests for browsers and HTTP 1.0 proxies
      // (see https://stackoverflow.com/a/2068407)
      headers.insert(CACHE_CONTROL, header_value(PreBuiltHeader::DisableCache));
      headers.insert(EXPIRES, header_value(PreBuiltHeader::Zero));

      response
    },
    "ws" => ws(&subpath, req, communicator).await,
    _ => status_response(StatusCode::NOT_FOUND)
  }
}

#[cfg(feature = "secure_server")]
fn load_secure_server_data() -> (Vec<Certificate>, PrivateKey) {
  let certs_path = &SETTINGS.secure_server.cert_path;
  let keys_path = &SETTINGS.secure_server.key_path;

  let certs_file = File::open(certs_path).unwrap_or_else(|err| {
    exit_with_error(&format!("Open certs file \"{certs_path}\" error: {err}"))
  });
  let keys_file = File::open(keys_path).unwrap_or_else(|err| {
    exit_with_error(&format!("Open keys file \"{keys_path}\" error: {err}"))
  });

  let certs_raw = certs(&mut BufReader::new(certs_file)).unwrap_or_else(|err| {
    exit_with_error(&format!("Extract certs from \"{certs_path}\" error: {err}"))
  });
  let keys_raw = rsa_private_keys(&mut BufReader::new(keys_file)).unwrap_or_else(|err| {
    exit_with_error(&format!("Extract keys from \"{keys_path}\" error: {err}"))
  });

  let certs = certs_raw.into_iter().map(Certificate).collect::<Vec<Certificate>>();
  let keys = keys_raw.into_iter().map(PrivateKey).collect::<Vec<PrivateKey>>();

  let key = keys.get(0).unwrap_or_else(|| {
    exit_with_error("Keys file does not contain any key")
  }).clone();

  (certs, key)
}

#[cfg(feature = "secure_server")]
fn create_additional_acceptor() -> Arc<Mutex<TlsAcceptor>> {
  let (certs, key) = load_secure_server_data();

  let server_config = ServerConfig::builder()
    .with_safe_defaults()
    .with_no_client_auth()
    .with_single_cert(certs, key)
    .unwrap_or_else(|err| {
      exit_with_error(&format!("Create TLS server config error: {err}"))
    });

  Arc::new(Mutex::new(TlsAcceptor::from(Arc::new(server_config))))
}

#[cfg(not(feature = "secure_server"))]
const fn create_additional_acceptor() {}

async fn create_tcp_listener(addr: SocketAddr) -> TcpListener {
  let listener = TcpListener::bind(addr).await.unwrap_or_else(|err| {
    exit_with_error(&format!("Create address listener error: {err}"))
  });

  info!("Listening on http://{addr}");

  listener
}

async fn accept_connection(listener: &TcpListener) -> Option<(TcpStream, SocketAddr)> {
  let accept_result = listener.accept().await;

  if let Err(err) = &accept_result {
    debug!("Accept TCP connection error: {err}");
  }

  accept_result.ok()
}

async fn serve_connection<I>(stream: I, service: Service)
where
  I: AsyncRead + AsyncWrite + Unpin + Send + 'static
{
  Builder::new()
    .max_buf_size(MAX_HTTP_BODY_USIZE)
    .serve_connection(stream, service)
    .with_upgrades()
    .await
    .unwrap_or_else(|err| debug!("Handle connection error: {err}"));
}

#[cfg(feature = "secure_server")]
async fn run(
  listener: TcpListener, service: Service,
  acceptor: Arc<Mutex<TlsAcceptor>>, mut stop_receiver: Receiver<()>
) {
  loop {
    select! {
      Some((stream, _)) = accept_connection(&listener) => {
        let acceptor_clone = acceptor.clone();
        let service_clone = service.clone();

        spawn(async move {
          let acceptor = acceptor_clone.lock().await;
          let stream_result = acceptor.accept(stream).await;
          drop(acceptor);

          let stream = match stream_result {
            Ok(stream) => stream,
            Err(err) => {
              debug!("Accept TLS connection error: {err}");
              return
            }
          };

          serve_connection(stream, service_clone).await;
        });
      },
      _ = &mut stop_receiver => {
        debug!("Graceful http shutdown");
        break
      }
    }
  }
}

#[cfg(not(feature = "secure_server"))]
async fn run(listener: TcpListener, service: Service, _: (), mut stop_receiver: Receiver<()>) {
  loop {
    select! {
      Some((stream, _)) = accept_connection(&listener) => {
        spawn(serve_connection(stream, service.clone()));
      },
      _ = &mut stop_receiver => {
        debug!("Graceful http shutdown");
        break
      }
    }
  }
}

pub async fn start(communicator: Arc<Mutex<Communicator>>, stop_receiver: Receiver<()>) {
  // For "secure_server" feature create_additional_acceptor return used later value,
  // it used for same `run` function signatures for "secure_server" and if it disabled
  #[allow(clippy::let_unit_value)]
  let additional_acceptor = create_additional_acceptor();
  let listener = create_tcp_listener(SETTINGS.bind_addr).await;

  // Initialize lazy static refs before server start accept connections
  // to prevent slowdown first requests
  initialize(&WEB_SOCKET_CONFIG);
  initialize(&HEADER_VALUES);
  initialize(&MIME_TYPES);
  initialize(&ROUTE_HANDLERS);
  #[cfg(any(feature = "client_resources_caching", feature = "client_resources_packing"))]
  initialize(&CLIENT_RESOURCES);

  run(listener, Service { communicator }, additional_acceptor, stop_receiver).await;

  // TODO: when https://github.com/hyperium/hyper/issues/2730 will be fixed,
  //       implement server graceful shutdown
}
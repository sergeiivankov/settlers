use bytes::Bytes;
use http_body_util::Full;
use hyper::{
  Request, Response, StatusCode, body::Incoming, server::conn::http1::Builder, service::service_fn
};
use lazy_static::initialize;
use log::{ debug, info, error };
use std::{ marker::Unpin, net::SocketAddr, sync::Arc };
use tokio::{
  io::{ AsyncRead, AsyncWrite }, net::{ TcpListener, TcpStream },
  sync::{ oneshot::Receiver, Mutex }, task::spawn, select
};
use crate::{ communicator::Communicator, helpers::{ exit_with_error, get_env } };
use super::{
  api::api,
  helpers::{ CURRENT_PATH, create_status_response },
  serve::{ PUBLIC_RESOURCES_PATH, serve },
  ws::ws
};

#[cfg(feature = "public_resources_caching")]
use super::serve::PUBLIC_RESOURCES_CACHE;

#[cfg(feature = "secure_server")]
use rustls_pemfile::{ certs, rsa_private_keys };
#[cfg(feature = "secure_server")]
use tokio_rustls::{ rustls::{ Certificate, PrivateKey, ServerConfig }, TlsAcceptor };
#[cfg(feature = "secure_server")]
use std::{ fs::File, io::BufReader, sync::Arc };
#[cfg(feature = "secure_server")]
use tokio::sync::Mutex;
#[cfg(feature = "secure_server")]
use super::helpers::prepare_check_path;

#[cfg(feature = "secure_server")]
fn load_secure_server_data() -> (Vec<Certificate>, PrivateKey) {
  let certs_path = prepare_check_path(get_env("SETTLERS_CERT_PATH"), true);
  let keys_path = prepare_check_path(get_env("SETTLERS_KEY_PATH"), true);

  let certs_file = File::open(&certs_path).unwrap_or_else(|err| {
    exit_with_error(format!("Open certs file \"{}\" error: {}", certs_path, err))
  });
  let keys_file = File::open(&keys_path).unwrap_or_else(|err| {
    exit_with_error(format!("Open keys file \"{}\" error: {}", keys_path, err))
  });

  let mut certs_raw = certs(&mut BufReader::new(certs_file)).unwrap_or_else(|err| {
    exit_with_error(format!("Extract certs from \"{}\" error: {}", certs_path, err))
  });
  let mut keys_raw = rsa_private_keys(&mut BufReader::new(keys_file)).unwrap_or_else(|err| {
    exit_with_error(format!("Extract keys from \"{}\" error: {}", keys_path, err))
  });

  let certs = certs_raw.drain(..).map(Certificate).collect::<Vec<Certificate>>();
  let keys = keys_raw.drain(..).map(PrivateKey).collect::<Vec<PrivateKey>>();

  let key = keys.get(0).unwrap_or_else(|| {
    exit_with_error(format!("Keys file does not contain any key"))
  }).clone();

  (certs, key)
}

async fn handle_connection(
  req: Request<Incoming>, communicator: Arc<Mutex<Communicator>>
) -> Result<Response<Full<Bytes>>, String> {
  let uri = req.uri().clone();

  let path = match uri.path().get(1..) {
    Some(path) => path,
    None => ""
  };

  let (section, subpath) = {
    match path.split_once("/") {
      Some(parts) => parts,
      None => if path == "" { ("public", "index.html") } else { (path, "") }
    }
  };

  match section {
    "public" => serve(subpath, req).await,
    "api" => api(subpath, req).await,
    "ws" => ws(subpath, req, communicator).await,
    _ => create_status_response(StatusCode::NOT_FOUND)
  }
}

async fn create_tcp_listener(addr: SocketAddr) -> TcpListener {
  let listener = TcpListener::bind(addr).await.unwrap_or_else(|err| {
    exit_with_error(format!("Create address listener error: {}", err))
  });

  info!("Listening on http://{}", addr);

  listener
}

async fn accept_connection(listener: &TcpListener) -> Option<(TcpStream, SocketAddr)> {
  match listener.accept().await {
    Ok(connection) => Some(connection),
    Err(err) => {
      debug!("Accept TCP connection error: {}", err);
      None
    }
  }
}

async fn serve_connection<I>(stream: I, communicator: Arc<Mutex<Communicator>>)
where
  I: AsyncRead + AsyncWrite + Unpin + Send + 'static
{
  let connection = Builder::new().serve_connection(
    stream, service_fn(move |req| handle_connection(req, communicator.clone()))
  );
  let connection = connection.with_upgrades();
  connection.await.unwrap_or_else(|err| error!("Handle connection error: {}", err));
}

#[cfg(feature = "secure_server")]
fn create_additional_acceptor() -> Arc<Mutex<TlsAcceptor>> {
  let (certs, key) = load_secure_server_data();

  let server_config = ServerConfig::builder()
    .with_safe_defaults()
    .with_no_client_auth()
    .with_single_cert(certs, key)
    .unwrap_or_else(|err| {
      exit_with_error(format!("Create TLS server config error: {}", err))
    });

  Arc::new(Mutex::new(TlsAcceptor::from(Arc::new(server_config))))
}

#[cfg(not(feature = "secure_server"))]
fn create_additional_acceptor() {}

#[cfg(feature = "secure_server")]
async fn run(
  listener: TcpListener, communicator: Arc<Mutex<Communicator>>,
  acceptor: Arc<Mutex<TlsAcceptor>>, mut stop_receiver: Receiver<()>
) {
  loop {
    select! {
      Some((stream, _)) = accept_connection(&listener) => {
        let acceptor_clone = acceptor.clone();

        spawn(async move {
          let acceptor = acceptor_clone.lock().await;

          let stream = match acceptor.accept(stream).await {
            Ok(stream) => stream,
            Err(err) => {
              debug!("Accept TLS connection error: {}", err);
              return
            }
          };

          drop(acceptor);

          serve_connection(stream, communicator.clone()).await
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
async fn run(
  listener: TcpListener, communicator: Arc<Mutex<Communicator>>,
  _acceptor: (), mut stop_receiver: Receiver<()>
) {
  loop {
    select! {
      Some((stream, _)) = accept_connection(&listener) => {
        spawn(serve_connection(stream, communicator.clone()));
      },
      _ = &mut stop_receiver => {
        debug!("Graceful http shutdown");
        break
      }
    }
  }
}

pub async fn start(communicator: Arc<Mutex<Communicator>>, stop_receiver: Receiver<()>) {
  // Need to check for errors lazy static refs before server start
  initialize(&CURRENT_PATH);
  initialize(&PUBLIC_RESOURCES_PATH);

  let addr_string = get_env("SETTLERS_BIND_ADDR");

  let addr: SocketAddr = addr_string.parse().unwrap_or_else(|err| {
    exit_with_error(format!("Parse bind address \"{}\" error: {}", addr_string, err))
  });

  let additional_acceptor = create_additional_acceptor();
  let listener = create_tcp_listener(addr).await;

  // Initialize public resources cache before server start accept connections
  #[cfg(feature = "public_resources_caching")]
  initialize(&PUBLIC_RESOURCES_CACHE);

  run(listener, communicator, additional_acceptor, stop_receiver).await;

  // TODO: server graceful shutdown
  // In hyper-1.0.0-rc.2 not resolved some issues related with
  // server graceful shutdown, so its not used here
}
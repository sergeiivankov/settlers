use bytes::Bytes;
use http_body_util::Full;
use hyper::{
  Request, Response, StatusCode, body::Incoming, server::conn::http1::Builder, service::service_fn
};
use lazy_static::initialize;
use log::{ info, error };
use rustls_pemfile::{ certs, rsa_private_keys };
use std::{ fs::File, io::BufReader, marker::Unpin, net::SocketAddr, sync::Arc };
use tokio::{
  io::{ AsyncRead, AsyncWrite }, net::{ TcpListener, TcpStream },
  signal::ctrl_c, task::spawn, select, sync::Mutex
};
use tokio_rustls::{ rustls::{ Certificate, PrivateKey, ServerConfig }, TlsAcceptor };
use crate::helpers::{ exit_with_error, get_env };
use super::{
  api::api,
  helpers::{ CURRENT_PATH, prepare_check_path, create_status_response },
  serve::{ PUBLIC_RESOURCES_PATH, serve },
  ws::ws
};

async fn handle_connection(req: Request<Incoming>) -> Result<Response<Full<Bytes>>, String> {
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
    "public" => serve(subpath).await,
    "api" => api(subpath, req).await,
    "ws" => ws(subpath, req).await,
    _ => create_status_response(StatusCode::NOT_FOUND)
  }
}

fn load_secure_server_data() -> Option<(Vec<Certificate>, PrivateKey)> {
  let cert_path_value = get_env("SETTLERS_CERT_PATH", false);
  let key_path_value = get_env("SETTLERS_KEY_PATH", false);

  if cert_path_value == "" || key_path_value == "" {
    return None
  }

  let certs_path = prepare_check_path(cert_path_value, true);
  let keys_path = prepare_check_path(key_path_value, true);

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

  Some((certs, key))
}

async fn create_tcp_listener(addr: SocketAddr) -> TcpListener {
  let listener = TcpListener::bind(addr).await.unwrap_or_else(|err| {
    exit_with_error(format!("Create address listener error: {}", err))
  });

  info!("Listening on http://{}", addr);

  listener
}

async fn accept_connection(listener: &TcpListener) -> Option<TcpStream> {
  match listener.accept().await {
    Ok(connection) => Some(connection.0),
    Err(err) => {
      error!("Accept TCP connection error: {}", err);
      None
    }
  }
}

async fn serve_connection<I>(stream: I)
where
  I: AsyncRead + AsyncWrite + Unpin + Send + 'static
{
  let connection = Builder::new().serve_connection(stream, service_fn(handle_connection));
  let connection = connection.with_upgrades();
  connection.await.unwrap_or_else(|err| error!("Handle HTTP connection error: {}", err));
}

pub async fn start() {
  // Need to check for errors lazy static refs before server start
  initialize(&CURRENT_PATH);
  initialize(&PUBLIC_RESOURCES_PATH);

  let addr_string = get_env("SETTLERS_BIND_ADDR", true);

  let addr: SocketAddr = addr_string.parse().unwrap_or_else(|err| {
    exit_with_error(format!("Parse bind address \"{}\" error: {}", addr_string, err))
  });

  let secure_server_data = load_secure_server_data();

  let listener = create_tcp_listener(addr).await;

  if let Some((certs, key)) = secure_server_data {
    let server_config = ServerConfig::builder()
      .with_safe_defaults()
      .with_no_client_auth()
      .with_single_cert(certs, key)
      .unwrap_or_else(|err| {
        exit_with_error(format!("Create TLS server config error: {}", err))
      });
    let acceptor = Arc::new(Mutex::new(TlsAcceptor::from(Arc::new(server_config))));

    loop {
      select! {
        Some(stream) = accept_connection(&listener) => {
          let acceptor_clone = acceptor.clone();

          spawn(async move {
            let acceptor = acceptor_clone.lock().await;

            let stream = match acceptor.accept(stream).await {
              Ok(stream) => stream,
              Err(err) => {
                error!("Accept TLS connection error: {}", err);
                return
              }
            };

            drop(acceptor);

            serve_connection(stream).await
          });
        },
        _ = ctrl_c() => break
      }
    }
  } else {
    loop {
      select! {
        Some(stream) = accept_connection(&listener) => {
          spawn(serve_connection(stream));
        },
        _ = ctrl_c() => break
      }
    }
  }

  // TODO: server graceful shutdown
  // In hyper-1.0.0-rc.2 not resolved some issues related with
  // server graceful shutdown, so its not used here
}
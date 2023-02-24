use bytes::Bytes;
use futures_util::{ SinkExt, StreamExt };
use http_body_util::Full;
use hyper::{
  body::Incoming,
  header::{
    CONNECTION, SEC_WEBSOCKET_ACCEPT, SEC_WEBSOCKET_KEY, SEC_WEBSOCKET_VERSION, UPGRADE,
    HeaderMap, HeaderName, HeaderValue
  },
  upgrade::{ Upgraded, on },
  Method, Request, Response, StatusCode, Version
};
use log::{ debug, error };
use std::sync::Arc;
use tokio::{ sync::Mutex, task::spawn, select };
use tokio_tungstenite::{
  WebSocketStream, tungstenite::{ handshake::derive_accept_key, protocol::Role, Error, Message }
};
use crate::communicator::Communicator;
use super::helpers::status_response;

fn get_header_str(name: HeaderName, headers: &HeaderMap) -> Option<&str> {
  match headers.get(name) {
    Some(value) => match value.to_str() {
      Ok(value) => Some(value),
      Err(_) => None
    },
    None => None
  }
}

async fn handle_connection(
  stream: WebSocketStream<Upgraded>, communicator: Arc<Mutex<Communicator>>
) {
  let mut communicator_lock = communicator.lock().await;
  let (id, sender, mut receiver) = communicator_lock.add();
  drop(communicator_lock);

  let (mut write, mut read) = stream.split();

  loop {
    select! {
      from = read.next() => {
        match from {
          Some(result) => match result {
            Ok(message) => match message {
              Message::Text(data) => match sender.send((id, data)) {
                Ok(_) => {},
                Err(err) => {
                  error!("Send from peer {} error: {}", id, err);
                  break
                }
              },
              _ => {}
            },
            Err(err) => {
              debug!("Receive WS message {} error: {}", id, err);
              break
            }
          },
          None => break
        }
      },
      to = receiver.recv() => {
        match to {
          Some(data) => {
            match write.send(Message::Text(data)).await {
              Ok(_) => {},
              Err(err) => {
                debug!("Send WS message {} error: {}", id, err);
                break
              }
            }
          },
          None => {
            error!("Sender to peer closed before it remove from communicator {}", id);
            break
          }
        }
      }
    }
  }

  let mut communicator_lock = communicator.lock().await;
  communicator_lock.remove(&id);
  drop(communicator_lock);

  let mut stream = match write.reunite(read) {
    Ok(stream) => stream,
    Err(err) => {
      error!("Reunite WS stream parts {} error: {}", id, err);
      return
    }
  };

  match stream.close(None).await {
    Ok(_) => {},
    Err(err) => match err {
      Error::ConnectionClosed => {},
      _ => error!("Close WS stream {} error: {}", id, err)
    }
  }
}

pub async fn ws(
  path: &str, mut req: Request<Incoming>, communicator: Arc<Mutex<Communicator>>
) -> Response<Full<Bytes>> {
  let version = req.version();
  let headers = req.headers();
  let key = headers.get(SEC_WEBSOCKET_KEY);

  if req.method() != Method::GET
  || version != Version::HTTP_11
  || path != ""
  || key.is_none()
  || get_header_str(CONNECTION, &headers)
       .map(|s| s.split(&[' ', ',']).any(|p| p.eq_ignore_ascii_case("upgrade")))
       .unwrap_or(false) == false
  || get_header_str(UPGRADE, &headers)
       .map(|s| s.eq_ignore_ascii_case("websocket"))
       .unwrap_or(false) == false
  || headers.get(SEC_WEBSOCKET_VERSION).map(|v| v == "13").unwrap_or(false) == false
  {
    debug!("Check creating WS connection error: {:?}", req);
    return status_response(StatusCode::BAD_REQUEST)
  }

  let derived = derive_accept_key(key.unwrap().as_bytes());

  spawn(async move {
    match on(&mut req).await {
      Ok(upgraded) => handle_connection(
        WebSocketStream::from_raw_socket(upgraded, Role::Server, None).await, communicator
      ).await,
      Err(err) => debug!("Upgrade HTTP connection error: {}", err)
    }
  });

  let mut response = Response::new(Full::new(Bytes::new()));
  *response.version_mut() = version;
  *response.status_mut() = StatusCode::SWITCHING_PROTOCOLS;

  let headers = response.headers_mut();
  headers.insert(CONNECTION, HeaderValue::from_str("Upgrade").unwrap());
  headers.insert(UPGRADE, HeaderValue::from_str("websocket").unwrap());
  headers.insert(SEC_WEBSOCKET_ACCEPT, HeaderValue::from_str(&derived).unwrap());

  response
}
use bytes::Bytes;
use futures_util::{ SinkExt, StreamExt };
use http_body_util::Full;
use hyper::{
  body::Incoming,
  header::{
    CONNECTION, SEC_WEBSOCKET_ACCEPT, SEC_WEBSOCKET_KEY, SEC_WEBSOCKET_VERSION, UPGRADE, HeaderValue
  },
  upgrade::{ Upgraded, on },
  Method, Request, Response, StatusCode, Version
};
use log::{ debug, error };
use std::sync::Arc;
use tokio::{ sync::Mutex, task::spawn, select };
use tokio_tungstenite::{
  tungstenite::{ handshake::derive_accept_key, protocol::Role, Error, Message }, WebSocketStream
};
use crate::communicator::Communicator;
use super::helpers::{
  WEB_SOCKET_CONFIG, HttpResponse, PreBuiltHeader,
  header_value, get_header_str, header_list_contains, status_response
};

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
            Ok(message) => if let Message::Text(data) = message {
              if let Err(err) = sender.send((id, data)) {
                error!("Send from peer {id} error: {err}");
                break
              }
            },
            Err(err) => {
              debug!("Receive WS message {id} error: {err}");
              break
            }
          },
          None => break
        }
      },
      to = receiver.recv() => {
        if let Some(data) = to {
          if let Err(err) = write.send(Message::Text(data)).await {
            debug!("Send WS message {id} error: {err}");
            break
          }
        } else {
          error!("Sender to peer closed before it remove from communicator {id}");
          break
        }
      }
    }
  }

  let mut communicator_lock = communicator.lock().await;
  communicator_lock.remove(id);
  drop(communicator_lock);

  let mut stream = match write.reunite(read) {
    Ok(stream) => stream,
    Err(err) => {
      error!("Reunite WS stream parts {id} error: {err}");
      return
    }
  };

  if let Err(err) = stream.close(None).await {
    if !matches!(err, Error::ConnectionClosed) {
      error!("Close WS stream {id} error: {err}");
    }
  }
}

pub async fn ws(
  path: &str, mut req: Request<Incoming>, communicator: Arc<Mutex<Communicator>>
) -> HttpResponse {
  let version = req.version();
  let headers = req.headers();
  let key_option = headers.get(SEC_WEBSOCKET_KEY);

  if req.method() != Method::GET
  || version != Version::HTTP_11
  || key_option.is_none()
  || !path.is_empty()
  || !headers.get(SEC_WEBSOCKET_VERSION).map_or(false, |v| v == "13")
  || !get_header_str(headers, &UPGRADE).map_or(false, |s| s.eq_ignore_ascii_case("websocket"))
  || !header_list_contains(headers, &CONNECTION, "upgrade")
  {
    debug!("Check creating WS connection error: {req:?}");
    return status_response(StatusCode::BAD_REQUEST)
  }

  // SAFETY: in previous condition block check that key is None and in this case function terminates
  let key = unsafe { key_option.unwrap_unchecked() };
  let derived = derive_accept_key(key.as_bytes());

  spawn(async move {
    match on(&mut req).await {
      Ok(upgraded) => handle_connection(
        WebSocketStream::from_raw_socket(upgraded, Role::Server, Some(*WEB_SOCKET_CONFIG)).await,
        communicator
      ).await,
      Err(err) => debug!("Upgrade HTTP connection error: {err}")
    }
  });

  let mut response = Response::new(Full::new(Bytes::new()));

  *response.version_mut() = version;
  *response.status_mut() = StatusCode::SWITCHING_PROTOCOLS;

  let headers = response.headers_mut();
  headers.insert(CONNECTION, header_value(PreBuiltHeader::Upgrade));
  headers.insert(UPGRADE, header_value(PreBuiltHeader::WebSocket));

  let accept_value_result = HeaderValue::from_str(&derived);
  // SAFETY: derived contains only base64 standart alphabet symbols,
  //         which are valid header value characters
  let accept_value = unsafe { accept_value_result.unwrap_unchecked() };
  headers.insert(SEC_WEBSOCKET_ACCEPT, accept_value);

  response
}
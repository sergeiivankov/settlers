use bytes::Bytes;
use futures_util::{ SinkExt, StreamExt };
use http_body_util::Full;
use hyper::{
  Method, Request, Response, StatusCode, Version, body::Incoming,
  header::{
    CONNECTION, SEC_WEBSOCKET_ACCEPT, SEC_WEBSOCKET_KEY, SEC_WEBSOCKET_VERSION, UPGRADE,
    HeaderMap, HeaderName
  },
  upgrade::{ Upgraded, on }
};
use log::debug;
use tokio::task::spawn;
use tokio_tungstenite::{
  WebSocketStream, tungstenite::{ handshake::derive_accept_key, protocol::Role }
};
use super::helpers::{ create_status_response, return_result_response };

fn get_header_str(name: HeaderName, headers: &HeaderMap) -> Option<&str> {
  match headers.get(name) {
    Some(value) => match value.to_str() {
      Ok(value) => Some(value),
      Err(_) => None
    },
    None => None
  }
}

async fn handle_connection(stream: WebSocketStream<Upgraded>) {
  let (mut write, mut read) = stream.split();

  while let Some(result) = read.next().await {
    let message = match result {
      Ok(message) => message,
      Err(err) => {
        debug!("Receive WS message error: {}", err);
        return
      }
    };

    match write.send(message).await {
      Ok(_) => {},
      Err(err) => {
        debug!("Send WS message error: {}", err);
        return
      }
    }
  }
}

pub async fn ws(path: &str, mut req: Request<Incoming>) -> Result<Response<Full<Bytes>>, String> {
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
    return create_status_response(StatusCode::NOT_FOUND)
  }

  let derived = derive_accept_key(key.unwrap().as_bytes());

  spawn(async move {
    match on(&mut req).await {
      Ok(upgraded) => handle_connection(
        WebSocketStream::from_raw_socket(upgraded, Role::Server, None).await
      ).await,
      Err(err) => debug!("Upgrade HTTP connection error: {}", err)
    }
  });

  return_result_response(
    Response::builder()
      .version(version)
      .status(StatusCode::SWITCHING_PROTOCOLS)
      .header(CONNECTION, "Upgrade")
      .header(UPGRADE, "websocket")
      .header(SEC_WEBSOCKET_ACCEPT, derived)
      .body(Full::new(Bytes::new()))
  )
}
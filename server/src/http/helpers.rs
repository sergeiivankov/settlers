use bytes::Bytes;
use http_body_util::Full;
use hyper::{ header::HeaderValue, Response, StatusCode };
use lazy_static::lazy_static;
use std::collections::HashMap;
use strum::{ AsRefStr, EnumIter, IntoEnumIterator };
use tokio_tungstenite::tungstenite::protocol::WebSocketConfig;
use crate::helpers::exit_with_error;

pub type HttpResponse = Response<Full<Bytes>>;

// Maximum HTTP body size
// Maximum client payload would be a profile picture upload,
// so 128 KiB should be enough for 160x160 png image, cropped by circle
pub const MAX_HTTP_BODY_SIZE: u64 = 128 * 1024;

// Maximum API request HTTP body size
// IMPORTANT: for profile picture upload method use main HTTP body limit
pub const MAX_API_BODY_SIZE: u64 = 1024;

// Maximum WebSocket message size
// In the future, it can be increased depending on the maximum size
// of data transmitted in one message
const MAX_WEB_SOCKET_MESSAGE_SIZE: usize = 1024;

lazy_static! {
  pub static ref HEADER_VALUES: HashMap<PreBuiltHeader, HeaderValue> = {
    let header_keys = PreBuiltHeader::iter();
    let mut header_values = HashMap::with_capacity(header_keys.len());

    for key in header_keys {
      header_values.insert(key, build_header_value(key.as_ref()));
    }

    header_values
  };

  pub static ref MIME_TYPES: HashMap<&'static str, HeaderValue> = {
    // IMPORTANT: increase capacity when new mime type will be added
    let mut mime_types = HashMap::with_capacity(5);

    mime_types.insert("html", build_header_value("text/html"));
    mime_types.insert("js", build_header_value("text/javascript"));
    mime_types.insert("css", build_header_value("text/css"));
    mime_types.insert("png", build_header_value("image/png"));
    mime_types.insert("wasm", build_header_value("application/wasm"));

    mime_types
  };

  pub static ref WEB_SOCKET_CONFIG: WebSocketConfig = WebSocketConfig {
    max_send_queue: None,
    max_message_size: Some(MAX_WEB_SOCKET_MESSAGE_SIZE),
    max_frame_size: Some(MAX_WEB_SOCKET_MESSAGE_SIZE),
    accept_unmasked_frames: false
  };
}

fn build_header_value(value: &str) -> HeaderValue {
  HeaderValue::from_str(value)
    .unwrap_or_else(|_| exit_with_error(format!("Create header value error: \"{}\"", value)))
}

#[derive(Copy, Clone, Eq, PartialEq, Hash, AsRefStr, EnumIter)]
#[repr(u8)]
pub enum PreBuiltHeader {
  #[strum(serialize = "application/octet-stream")]
  AppOctetStream,
  #[strum(serialize = "no-store, must-revalidate")]
  DisableCache,
  #[strum(serialize = "Upgrade")]
  Upgrade,
  #[strum(serialize = "websocket")]
  WebSocket,
  #[strum(serialize = "0")]
  Zero
}

pub fn header_value(key: PreBuiltHeader) -> HeaderValue {
  let value_option = HEADER_VALUES.get(&key);
  // SAFETY: to build HEADER_VALUES used iterator over PreBuiltHeader variants,
  //         so HashMap contain them all as keys
  let value = unsafe { value_option.unwrap_unchecked() };
  value.clone()
}

pub fn status_response(code: StatusCode) -> HttpResponse {
  let reason_phrase_option = code.canonical_reason();
  // SAFETY: all provided by hyper status codes have standardised reason phrase
  let reason_phrase = unsafe { reason_phrase_option.unwrap_unchecked() };

  let mut response = Response::new(reason_phrase.into());
  *response.status_mut() = code;

  response
}
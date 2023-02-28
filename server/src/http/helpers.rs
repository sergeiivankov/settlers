use bytes::{ BufMut, Bytes, BytesMut };
use http_body_util::Full;
use hyper::{ header::{ CONTENT_TYPE, HeaderValue }, Response, StatusCode };
use lazy_static::lazy_static;
use log::debug;
use quick_protobuf::{ BytesReader, MessageRead, MessageWrite, Writer };
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
  pub static ref HEADER_VALUES: HashMap<u8, HeaderValue> = {
    let header_keys = PreBuiltHeader::iter();
    let mut header_values = HashMap::with_capacity(header_keys.len());

    for key in header_keys {
      header_values.insert(key as u8, build_header_value(key.as_ref()));
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

#[derive(Copy, Clone, AsRefStr, EnumIter)]
#[repr(u8)]
pub enum PreBuiltHeader {
  #[strum(serialize = "application/octet-stream")]
  ApplicationOctetStream,
  #[strum(serialize = "no-store, must-revalidate")]
  DisableCache,
  #[strum(serialize = "text/plain")]
  TextPlain,
  #[strum(serialize = "Upgrade")]
  Upgrade,
  #[strum(serialize = "websocket")]
  WebSocket,
  #[strum(serialize = "0")]
  Zero
}

pub fn header_value(key: PreBuiltHeader) -> HeaderValue {
  let value_option = HEADER_VALUES.get(&(key as u8));
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
  response.headers_mut().insert(CONTENT_TYPE, header_value(PreBuiltHeader::TextPlain));

  response
}

pub fn deserialize_api_params<'a, R: MessageRead<'a>>(body: &'a Bytes) -> Result<R, HttpResponse> {
  let mut reader = BytesReader::from_bytes(body);
  R::from_reader(&mut reader, body).map_err(|err| {
    debug!("Read API params error: {}", err);
    status_response(StatusCode::BAD_REQUEST)
  })
}

pub fn serialize_api_response<W: MessageWrite>(result: W) -> HttpResponse {
  let mut writer = BytesMut::zeroed(result.get_size()).writer();

  let write_result = result.write_message(&mut Writer::new(&mut writer));
  // SAFETY: WriterBackend implements may return only UnexpectedEndOfBuffer Err variant,
  //         which mean that writer is not long enough, but we create buffer with correct length
  unsafe { write_result.unwrap_unchecked(); };

  let mut response = Response::new(Full::new(writer.into_inner().freeze()));
  response.headers_mut().insert(CONTENT_TYPE, header_value(PreBuiltHeader::ApplicationOctetStream));

  response
}
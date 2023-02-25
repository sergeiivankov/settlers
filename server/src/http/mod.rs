mod api;
mod core;
mod serve;
mod ws;

use bytes::Bytes;
use http_body_util::Full;
use hyper::{ Response, StatusCode };

pub type HttpResponse = Response<Full<Bytes>>;

pub fn status_response(code: StatusCode) -> HttpResponse {
  let mut response = Response::new(code.canonical_reason().unwrap_or("").into());
  *response.status_mut() = code;
  response
}

pub use self::core::start;
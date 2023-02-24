use bytes::Bytes;
use http_body_util::Full;
use hyper::{ Response, StatusCode };

pub fn status_response(code: StatusCode) -> Response<Full<Bytes>> {
  let mut response = Response::new(code.canonical_reason().unwrap_or("").into());
  *response.status_mut() = code;
  response
}
include!(concat!(env!("OUT_DIR"), "/protos/mod.rs"));

use bytes::{ Bytes, BytesMut };
use http_body_util::Full;
use hyper::{ Response, StatusCode };
use quick_protobuf::{
  MessageRead, MessageWrite, deserialize_from_slice, serialize_into_slice
};
use crate::http::{ HttpResponse, status_response };

pub fn bytes_to_params<'a, R: MessageRead<'a>>(bytes: &'a Bytes) -> Result<R, HttpResponse> {
  deserialize_from_slice(bytes).map_err(|_| status_response(StatusCode::BAD_REQUEST))
}

pub fn result_to_response<W: MessageWrite>(result: W) -> HttpResponse {
  let mut bytes = BytesMut::with_capacity(result.get_size());
  match serialize_into_slice(&result, &mut bytes) {
    Ok(_) => {},
    Err(_) => return status_response(StatusCode::INTERNAL_SERVER_ERROR)
  }
  Response::new(Full::new(bytes.freeze()))
}
include!(concat!(env!("OUT_DIR"), "/protos/mod.rs"));

use bytes::{ BufMut, Bytes, BytesMut };
use http_body_util::Full;
use hyper::{ Response, StatusCode };
use log::debug;
use quick_protobuf::{ BytesReader, MessageRead, MessageWrite, Writer };
use crate::http::{ HttpResponse, status_response };

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

  Response::new(Full::new(writer.into_inner().freeze()))
}
use bytes::{ Bytes, BytesMut };
use http_body_util::{ BodyExt, Full };
use hyper::{ body::Incoming, Request, Response, StatusCode };
use lazy_static::lazy_static;
use quick_protobuf::{
  MessageRead, MessageWrite, deserialize_from_slice, serialize_into_slice
};
use std::collections::HashMap;
use crate::protos::auth::{
  CheckAuthTokenParams, CheckAuthTokenResult,
  CheckAuthTokenTestParams, CheckAuthTokenTestResult
};
use super::helpers::status_response;

type Routes = HashMap<
  &'static str,
  Box<dyn Fn(&Bytes) -> Result<Response<Full<Bytes>>, Response<Full<Bytes>>> + Sync>
>;

lazy_static! {
  pub static ref ROUTES: Routes = {
    let mut routes: Routes = HashMap::with_capacity(2);

    routes.insert("check_token", Box::new(|bytes| Ok(check_token(bytes_to_params(bytes)?))));
    routes.insert("check_token_test", Box::new(|bytes| Ok(check_token_test(bytes_to_params(bytes)?))));

    routes
  };
}

pub async fn api(path: &str, req: Request<Incoming>) -> Response<Full<Bytes>> {
  if !ROUTES.contains_key(path) {
    return status_response(StatusCode::NOT_FOUND)
  }

  let collected = match req.collect().await {
    Ok(collected) => collected,
    Err(_) => return status_response(StatusCode::INTERNAL_SERVER_ERROR)
  };
  let bytes = collected.to_bytes();

  let route = unsafe { ROUTES.get(path).unwrap_unchecked() };

  match route(&bytes) {
    Ok(response) => response,
    Err(response) => response
  }
}

fn bytes_to_params<'a, R: MessageRead<'a>>(bytes: &'a Bytes) -> Result<R, Response<Full<Bytes>>> {
  deserialize_from_slice(bytes).map_err(|_| status_response(StatusCode::BAD_REQUEST))
}

fn result_to_response<W: MessageWrite>(result: W) -> Response<Full<Bytes>> {
  let mut bytes = BytesMut::with_capacity(result.get_size());
  match serialize_into_slice(&result, &mut bytes) {
    Ok(_) => {},
    Err(_) => return status_response(StatusCode::INTERNAL_SERVER_ERROR)
  }
  Response::new(Full::new(bytes.freeze()))
}

fn check_token(_params: CheckAuthTokenParams) -> Response<Full<Bytes>> {
  result_to_response(CheckAuthTokenResult { result: true })
}

fn check_token_test(_params: CheckAuthTokenTestParams) -> Response<Full<Bytes>> {
  result_to_response(CheckAuthTokenTestResult { result: true })
}
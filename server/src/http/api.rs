use bytes::Bytes;
use http_body_util::BodyExt;
use hyper::{ body::Incoming, Request, StatusCode };
use lazy_static::lazy_static;
use log::debug;
use std::collections::HashMap;
use crate::protos::{
  auth::{
    CheckAuthTokenParams, CheckAuthTokenResult,
    CheckAuthTokenTestParams, CheckAuthTokenTestResult
  },
  bytes_to_params, result_to_response
};
use super::{ HttpResponse, status_response };

type Routes = HashMap<
  &'static str,
  Box<dyn Fn(&Bytes) -> Result<HttpResponse, HttpResponse> + Sync>
>;

// Maximum API request HTTP body size
// Important: for profile picture upload method use main HTTP body limit
const MAX_API_BODY_SIZE: u64 = 1024;

lazy_static! {
  pub static ref ROUTES: Routes = {
    let mut routes: Routes = HashMap::with_capacity(2);

    routes.insert("check_token", Box::new(|bytes| Ok(check_token(bytes_to_params(bytes)?))));
    routes.insert("check_token_test", Box::new(|bytes| Ok(check_token_test(bytes_to_params(bytes)?))));

    routes
  };
}

fn check_token(_params: CheckAuthTokenParams) -> HttpResponse {
  result_to_response(CheckAuthTokenResult { result: true })
}

fn check_token_test(_params: CheckAuthTokenTestParams) -> HttpResponse {
  result_to_response(CheckAuthTokenTestResult { result: true })
}

pub async fn api(path: &str, req: Request<Incoming>, body_size: u64) -> HttpResponse {
  if !ROUTES.contains_key(path) {
    return status_response(StatusCode::NOT_FOUND)
  }

  // Check API request maximum body size before read it
  // TODO: if upload profile picture method name changed, change it too
  if path != "upload_picture" {
    if body_size > MAX_API_BODY_SIZE {
      debug!("API body too large: {} > {}", body_size, MAX_API_BODY_SIZE);
      return status_response(StatusCode::PAYLOAD_TOO_LARGE)
    }
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
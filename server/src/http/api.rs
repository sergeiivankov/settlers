use bytes::Bytes;
use http_body_util::BodyExt;
use hyper::{ body::Incoming, Request, StatusCode };
use lazy_static::lazy_static;
use log::debug;
use std::collections::HashMap;
use crate::protos::{
  auth::{ CheckTokenParams, CheckTokenResult, CheckTokenTestParams, CheckTokenTestResult },
  deserialize_api_params as deserialize, serialize_api_response as serialize
};
use super::helpers::{ MAX_API_BODY_SIZE, HttpResponse, status_response };

type HandlerWrapper = dyn Fn(&Bytes) -> Result<HttpResponse, HttpResponse> + Sync;
type RouteHandlers = HashMap<&'static str, Box<HandlerWrapper>>;

lazy_static! {
  pub static ref ROUTE_HANDLERS: RouteHandlers = {
    // IMPORTANT: increase capacity when new route will be added
    let mut routes: RouteHandlers = HashMap::with_capacity(2);

    routes.insert("check_token", Box::new(|body| Ok(check_token(deserialize(body)?))));
    routes.insert("check_token_test", Box::new(|body| Ok(check_token_test(deserialize(body)?))));

    routes
  };
}

fn check_token(_params: CheckTokenParams) -> HttpResponse {
  serialize(CheckTokenResult { result: true })
}

fn check_token_test(_params: CheckTokenTestParams) -> HttpResponse {
  serialize(CheckTokenTestResult { result: true })
}

pub async fn api(path: &str, req: Request<Incoming>, body_size: u64) -> HttpResponse {
  if !ROUTE_HANDLERS.contains_key(path) {
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
  let body = collected.to_bytes();

  debug!("Received HTTP body for \"{}\": {:?}", path, body);

  let route_handler_option = ROUTE_HANDLERS.get(path);
  // SAFETY: at start of function we checked that ROUTES contains passed `path` key
  let route_handler = unsafe { route_handler_option.unwrap_unchecked() };

  // Route handler return ready to send builded response wrapped in Result
  // Ok variant contain return value of exactly handler function
  // Err variant contain error API params deserialization
  // Due to use of a shorter syntax `?` in API handlers closure wrappers
  match route_handler(&body) {
    Ok(response) => response,
    Err(response) => response
  }
}
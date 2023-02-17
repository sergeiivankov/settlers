use bytes::Bytes;
use http_body_util::Full;
use hyper::{ Response, StatusCode, http::Error as HttpError, http::Result as HttpResult };
use log::error;

pub fn handle_create_response_error(err: HttpError) -> Result<Response<Full<Bytes>>, String> {
  error!("Build response error: {}", err);

  let response_build_result = Response::builder()
    .status(StatusCode::INTERNAL_SERVER_ERROR)
    .body(Full::new(StatusCode::INTERNAL_SERVER_ERROR.canonical_reason().unwrap_or("").into()));

  match response_build_result {
    Ok(response) => Ok(response),
    Err(err) => Err(format!("Build response in \"handle_create_response_error\" error: {}", err))
  }
}

pub fn create_status_response(code: StatusCode) -> Result<Response<Full<Bytes>>, String> {
  return_result_response(
    Response::builder()
      .status(code)
      .body(Full::new(code.canonical_reason().unwrap_or("").into()))
  )
}

pub fn return_result_response(
  response_build_result: HttpResult<Response<Full<Bytes>>>
) -> Result<Response<Full<Bytes>>, String> {
  match response_build_result {
    Ok(response) => Ok(response),
    Err(err) => handle_create_response_error(err)
  }
}
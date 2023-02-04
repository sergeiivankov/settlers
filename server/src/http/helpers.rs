use bytes::Bytes;
use http_body_util::Full;
use hyper::{ Response, StatusCode, http::Error as HttpError, http::Result as HttpResult };
use lazy_static::lazy_static;
use log::error;
use std::{ env::current_dir, path::{ Path, PathBuf } };
use crate::helpers::exit_with_error;

lazy_static! {
  pub static ref CURRENT_PATH: PathBuf = current_dir()
    .unwrap_or_else(|err| exit_with_error(format!("Get current path error: {}", err)));
}

pub fn prepare_check_path(path_string: String, must_be_file: bool) -> String {
  let mut path = Path::new(&path_string);

  let mut path_absolute: PathBuf;
  if path.is_relative() {
    path_absolute = CURRENT_PATH.clone();
    path_absolute.push(path_string);
    path = Path::new(&path_absolute);
  }

  // In my cases, always return NotFound error kind, so canonicalize - it also a check for existence
  let path = path.canonicalize().unwrap_or_else(|_| {
    exit_with_error(format!("Path \"{}\" not exists", path.display()))
  });

  if path.is_file() != must_be_file {
    if must_be_file {
      exit_with_error(format!("Path \"{}\" must point to file", path.display()))
    } else {
      exit_with_error(format!("Path \"{}\" must point to directory", path.display()))
    }
  }

  let path_str = path.to_str().unwrap_or_else(|| {
    exit_with_error(format!("Convert path \"{}\" to str error", path.display()))
  });

  String::from(path_str)
}

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

pub fn return_result_response(
  response_build_result: HttpResult<Response<Full<Bytes>>>
) -> Result<Response<Full<Bytes>>, String> {
  match response_build_result {
    Ok(response) => Ok(response),
    Err(err) => handle_create_response_error(err)
  }
}

pub fn create_status_response(code: StatusCode) -> Result<Response<Full<Bytes>>, String> {
  return_result_response(
    Response::builder()
      .status(code)
      .body(Full::new(code.canonical_reason().unwrap_or("").into()))
  )
}
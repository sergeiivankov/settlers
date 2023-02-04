use bytes::Bytes;
use http_body_util::Full;
use hyper::{ Response, StatusCode };
use lazy_static::lazy_static;
use log::{ debug, error };
use std::{
  collections::HashMap, io::{ Error, ErrorKind }, path::{ MAIN_SEPARATOR, Component, Path, PathBuf }
};
use tokio::fs::read;
use crate::helpers::get_env;
use super::helpers::{ prepare_check_path, return_result_response, create_status_response };

#[cfg(feature = "public_caching")]
use async_recursion::async_recursion;
#[cfg(feature = "public_caching")]
use tokio::sync::Mutex;

lazy_static! {
  pub static ref PUBLIC_RESOURCES_PATH: String = prepare_check_path(
    get_env("SETTLERS_PUBLIC_RESOURCES_PATH"), false
  );

  static ref MIME_TYPES: HashMap<&'static str, &'static str> = {
    let mut mime_types = HashMap::new();
    mime_types.insert("html", "text/html");
    mime_types.insert("js", "text/javascript");
    mime_types.insert("css", "text/css");
    mime_types.insert("png", "image/png");
    mime_types.insert("wasm", "application/wasm");
    mime_types
  };
}

#[cfg(feature = "public_caching")]
lazy_static! {
  static ref RESOURCES_CACHE: Mutex<HashMap<String, Full<Bytes>>> = Mutex::new(HashMap::new());
}

fn get_full_file_path(path: &String) -> String {
  format!("{}{}{}", *PUBLIC_RESOURCES_PATH, MAIN_SEPARATOR, path)
}

#[cfg(feature = "public_caching")]
#[async_recursion]
async fn get_file_body(path: String) -> Result<Full<Bytes>, Error> {
  let mut cache = RESOURCES_CACHE.lock().await;

  match cache.get(&path) {
    Some(body) => Ok(body.clone()),
    None => {
      let full_path = get_full_file_path(&path);

      match read(full_path).await {
        Ok(content) => {
          let body = Full::new(content.into());
          cache.insert(path.clone(), body.clone());
          drop(cache);

          get_file_body(path).await
        },
        Err(err) => Err(err)
      }
    }
  }
}

#[cfg(not(feature = "public_caching"))]
async fn get_file_body(path: String) -> Result<Full<Bytes>, Error> {
  let full_path = get_full_file_path(&path);

  match read(full_path).await {
    Ok(content) => Ok(Full::new(content.into())),
    Err(err) => Err(err)
  }
}

pub async fn serve(path: &str) -> Result<Response<Full<Bytes>>, String> {
  // Path analisis for special components exists
  let path = {
    let mut normalized = PathBuf::new();

    for component in Path::new(path).components() {
      match component {
        Component::Prefix(_) | Component::CurDir | Component::RootDir | Component::ParentDir => {
          return create_status_response(StatusCode::NOT_FOUND)
        },
        Component::Normal(c) => normalized.push(c)
      };
    }

    normalized
  };

  let path = match path.to_str() {
    Some(path) => path,
    None => {
      error!("Convert path \"{}\" to str error", path.display());
      return create_status_response(StatusCode::INTERNAL_SERVER_ERROR)
    }
  };

  let path = String::from(path);

  let ext = match path.rsplit_once('.') {
    Some(parts) => parts.1,
    None => ""
  };
  let mime_type = MIME_TYPES.get(ext).unwrap_or(&"application/octet-stream");

  match get_file_body(path).await {
    Ok(body) => return_result_response(
      Response::builder().header("Content-Type", *mime_type).body(body.clone())
    ),
    Err(err) => {
      debug!("Read file error: {}", err);

      create_status_response(match err.kind() {
        ErrorKind::NotFound | ErrorKind::PermissionDenied => StatusCode::NOT_FOUND,
        _ => StatusCode::INTERNAL_SERVER_ERROR
      })
    }
  }
}
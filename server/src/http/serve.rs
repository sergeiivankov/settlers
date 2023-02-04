use async_recursion::async_recursion;
use bytes::Bytes;
use http_body_util::Full;
use hyper::{ Response, StatusCode };
use lazy_static::lazy_static;
use log::{ debug, error };
use std::{
  collections::HashMap, io::ErrorKind, path::{ MAIN_SEPARATOR, Component, Path, PathBuf }
};
use tokio::{ fs::read, sync::Mutex };
use crate::helpers::get_env;
use super::helpers::{ prepare_check_path, return_result_response, create_status_response };

lazy_static! {
  pub static ref PUBLIC_RESOURCES_PATH: String = prepare_check_path(
    get_env("SETTLERS_PUBLIC_RESOURCES_PATH"), false
  );

  static ref RESOURCES_CACHE: Mutex<HashMap<String, Full<Bytes>>> = Mutex::new(HashMap::new());

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

#[async_recursion]
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

  let mut cache = RESOURCES_CACHE.lock().await;

  match cache.get(&path) {
    Some(body) => {
      return_result_response(
        Response::builder().header("Content-Type", *mime_type).body(body.clone())
      )
    },
    None => {
      let full_path = format!("{}{}{}", *PUBLIC_RESOURCES_PATH, MAIN_SEPARATOR, path);

      match read(full_path).await {
        Ok(content) => {
          let body = Full::new(content.into());
          cache.insert(path.clone(), body.clone());
          drop(cache);

          serve(&path).await
        },
        Err(err) => {
          debug!("Read file error: {}", err);

          create_status_response(match err.kind() {
            ErrorKind::NotFound | ErrorKind::PermissionDenied => StatusCode::NOT_FOUND,
            _ => StatusCode::INTERNAL_SERVER_ERROR
          })
        }
      }
    }
  }
}
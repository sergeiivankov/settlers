use bytes::Bytes;
use http_body_util::Full;
use hyper::{
  body::Incoming, header::CONTENT_TYPE, http::Result as HttpResult, Response, Request, StatusCode
};
use lazy_static::lazy_static;
use log::debug;
use std::{
  collections::HashMap, io::{ Error, ErrorKind }, path::{ Component, Path, PathBuf }
};
use crate::helpers::get_env;
use super::helpers::{ prepare_check_path, return_result_response, create_status_response };

#[cfg(not(feature = "public_resources_caching"))]
use std::path::MAIN_SEPARATOR;
#[cfg(not(feature = "public_resources_caching"))]
use tokio::fs::read;

#[cfg(feature = "public_resources_caching")]
use hex::encode;
#[cfg(feature = "public_resources_caching")]
use hyper::header::{ ETAG, IF_NONE_MATCH };
#[cfg(feature = "public_resources_caching")]
use log::error;
#[cfg(feature = "public_resources_caching")]
use sha1::{ Sha1, Digest };
#[cfg(feature = "public_resources_caching")]
use std::fs::read;
#[cfg(feature = "public_resources_caching")]
use tokio::sync::Mutex;
#[cfg(feature = "public_resources_caching")]
use walkdir::WalkDir;

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

#[cfg(feature = "public_resources_caching")]
lazy_static! {
  // Read all public resources files to cache on server start
  // Values in HashMap is tuple with content hash and ready to return response body
  pub static ref PUBLIC_RESOURCES_CACHE: Mutex<HashMap<String, (String, Full<Bytes>)>> = {
    let mut cache = HashMap::new();

    let mut hasher = Sha1::new();

    for entry_result in WalkDir::new(&*PUBLIC_RESOURCES_PATH) {
      match entry_result {
        Ok(entry) => {
          let path = entry.path();
          if path.is_dir() {
            continue
          }

          let path_str = match path.to_str() {
            Some(path_str) => path_str,
            None => {
              error!("Convert path \"{}\" to str error", path.display());
              continue
            }
          };

          let content = match read(path) {
            Ok(content) => content,
            Err(err) => {
              error!("Read file error: {}", err);
              continue
            }
          };

          hasher.update(&content);
          let hash = hasher.finalize_reset();

          cache.insert(
            // Cut off path to public resources directory from full public resource path
            String::from(&path_str[(PUBLIC_RESOURCES_PATH.len() + 1)..]),
            (format!("\"{}\"", encode(hash)), Full::new(content.into()))
          );
        },
        Err(err) => error!("Walk entry error: {}", err)
      }
    }

    Mutex::new(cache)
  };
}

#[cfg(feature = "public_resources_caching")]
async fn get_response_data(
  path: String, mime_type: &str, req: Request<Incoming>
) -> Result<HttpResult<Response<Full<Bytes>>>, Error> {
  let cache = PUBLIC_RESOURCES_CACHE.lock().await;

  match cache.get(&path) {
    Some((hash, body)) => {
      if let Some(client_hash) = req.headers().get(IF_NONE_MATCH) {
        if client_hash == hash {
          return Ok(
            Response::builder().status(StatusCode::NOT_MODIFIED).body(Full::new(Bytes::new()))
          )
        }
      }

      Ok(Response::builder().header(CONTENT_TYPE, mime_type).header(ETAG, hash).body(body.clone()))
    },
    None => Err(Error::new(ErrorKind::NotFound, ""))
  }
}

#[cfg(not(feature = "public_resources_caching"))]
async fn get_response_data(
  path: String, mime_type: &str, _req: Request<Incoming>
) -> Result<HttpResult<Response<Full<Bytes>>>, Error> {
  let full_path = format!("{}{}{}", *PUBLIC_RESOURCES_PATH, MAIN_SEPARATOR, path);

  match read(full_path).await {
    Ok(content) => Ok(
      Response::builder().header(CONTENT_TYPE, mime_type).body(Full::new(content.into()))
    ),
    Err(err) => Err(err)
  }
}

pub async fn serve(path: &str, req: Request<Incoming>) -> Result<Response<Full<Bytes>>, String> {
  // Path analisis for special components exists
  let path = {
    let mut normalized = PathBuf::new();

    for component in Path::new(path).components() {
      match component {
        Component::Prefix(_) | Component::CurDir | Component::RootDir | Component::ParentDir => {
          debug!("Found special path component {:?} in \"{}\"", component, path);
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
      debug!("Convert path \"{}\" to str error", path.display());
      return create_status_response(StatusCode::INTERNAL_SERVER_ERROR)
    }
  };

  let path = String::from(path);

  let ext = match path.rsplit_once('.') {
    Some(parts) => parts.1,
    None => ""
  };
  let mime_type = MIME_TYPES.get(ext).unwrap_or(&"application/octet-stream");

  match get_response_data(path, mime_type, req).await {
    Ok(response) => return_result_response(response),
    Err(err) => {
      debug!("Read file error: {}", err);

      create_status_response(match err.kind() {
        ErrorKind::NotFound | ErrorKind::PermissionDenied => StatusCode::NOT_FOUND,
        _ => StatusCode::INTERNAL_SERVER_ERROR
      })
    }
  }
}
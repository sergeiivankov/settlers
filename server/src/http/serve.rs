use http_body_util::Full;
use hyper::{ body::Incoming, header::{ CONTENT_TYPE, HeaderValue }, Response, Request, StatusCode };
use lazy_static::lazy_static;
use log::debug;
use std::{
  collections::HashMap, io::{ Error, ErrorKind }, path::{ Component, Path, PathBuf }
};
use crate::settings::SETTINGS;
use super::{ HttpResponse, status_response };

#[cfg(not(feature = "public_resources_caching"))]
use std::path::MAIN_SEPARATOR;
#[cfg(not(feature = "public_resources_caching"))]
use tokio::fs::read;

#[cfg(feature = "public_resources_caching")]
use bytes::Bytes;
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
  pub static ref MIME_TYPES: HashMap<&'static str, &'static str> = {
    let mut mime_types = HashMap::with_capacity(5);
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

    for entry_result in WalkDir::new(&SETTINGS.public_resources_path) {
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
            String::from(&path_str[(SETTINGS.public_resources_path.len() + 1)..]),
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
) -> Result<HttpResponse, Error> {
  let cache = PUBLIC_RESOURCES_CACHE.lock().await;

  match cache.get(&path) {
    Some((hash, body)) => {
      if let Some(client_hash) = req.headers().get(IF_NONE_MATCH) {
        if client_hash == hash {
          let mut response = Response::new(Full::new(Bytes::new()));
          *response.status_mut() = StatusCode::NOT_MODIFIED;
          return Ok(response)
        }
      }

      let mut response = Response::new(body.clone());

      let headers = response.headers_mut();
      headers.insert(CONTENT_TYPE, HeaderValue::from_str(mime_type).unwrap());
      headers.insert(ETAG, HeaderValue::from_str(hash).unwrap());

      Ok(response)
    },
    None => Err(Error::new(ErrorKind::NotFound, ""))
  }
}

#[cfg(not(feature = "public_resources_caching"))]
async fn get_response_data(
  path: String, mime_type: &str, _: Request<Incoming>
) -> Result<HttpResponse, Error> {
  let full_path = format!("{}{}{}", SETTINGS.public_resources_path, MAIN_SEPARATOR, path);

  match read(full_path).await {
    Ok(content) => {
      let header_value = HeaderValue::from_str(mime_type).unwrap();

      let mut response = Response::new(Full::new(content.into()));
      response.headers_mut().insert(CONTENT_TYPE, header_value);

      Ok(response)
    },
    Err(err) => Err(err)
  }
}

pub async fn serve(path: &str, req: Request<Incoming>) -> HttpResponse {
  // Path analisis for special components exists
  let path = {
    let mut normalized = PathBuf::new();

    for component in Path::new(path).components() {
      match component {
        Component::Prefix(_) | Component::CurDir | Component::RootDir | Component::ParentDir => {
          debug!("Found special path component {:?} in \"{}\"", component, path);
          return status_response(StatusCode::NOT_FOUND)
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
      return status_response(StatusCode::INTERNAL_SERVER_ERROR)
    }
  };

  let path = String::from(path);

  let ext = match path.rsplit_once('.') {
    Some(parts) => parts.1,
    None => ""
  };
  let mime_type = MIME_TYPES.get(ext).unwrap_or(&"application/octet-stream");

  match get_response_data(path, mime_type, req).await {
    Ok(response) => response,
    Err(err) => {
      debug!("Read file error: {}", err);

      status_response(match err.kind() {
        ErrorKind::NotFound | ErrorKind::PermissionDenied => StatusCode::NOT_FOUND,
        _ => StatusCode::INTERNAL_SERVER_ERROR
      })
    }
  }
}
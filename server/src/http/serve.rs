use http_body_util::Full;
use hyper::{
  body::Incoming, header::{ CONTENT_TYPE, HeaderMap, HeaderValue }, Response, Request, StatusCode
};
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
#[cfg(feature = "public_resources_caching")]
use crate::helpers::exit_with_error;

lazy_static! {
  pub static ref MIME_TYPES: HashMap<&'static str, &'static str> = {
    // IMPORTANT: increase capacity when new mime type will be added
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
  // Values in HashMap is tuple with content hash ETAG HeaderValue and ready to return response body
  pub static ref PUBLIC_RESOURCES_CACHE: Mutex<HashMap<String, (HeaderValue, Full<Bytes>)>> = {
    let mut paths = Vec::new();

    for entry_result in WalkDir::new(&SETTINGS.public_resources_path) {
      match entry_result {
        Ok(entry) => {
          let path = entry.path().to_owned();
          if path.is_dir() {
            continue
          }

          paths.push(path);
        },
        Err(err) => error!("Walk entry error: {}", err)
      }
    }

    let mut cache = HashMap::with_capacity(paths.len());
    let mut hasher = Sha1::new();

    for path in &paths {
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

      let etag = HeaderValue::from_str(&format!("\"{}\"", encode(hash))).unwrap_or_else(|err| {
        exit_with_error(format!("Create \"Etag\" header error: {}", err))
      });

      cache.insert(
        // Cut off path to public resources directory from full public resource path
        String::from(&path_str[(SETTINGS.public_resources_path.len() + 1)..]),
        (etag, Full::new(content.into()))
      );
    }

    Mutex::new(cache)
  };
}

fn insert_mime_type(headers: &mut HeaderMap, mime_type: &str) {
  // mime_type variable may contain only "application/octet-stream" or values
  // from MIME_TYPES static ref, all these possible values not contain
  // invalid header value characters, so we can use unwrap_unchecked
  let mime_type_value = HeaderValue::from_str(mime_type);
  headers.insert(CONTENT_TYPE, unsafe { mime_type_value.unwrap_unchecked() });
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
      headers.insert(ETAG, hash.clone());
      insert_mime_type(headers, mime_type);

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
      let mut response = Response::new(Full::new(content.into()));
      insert_mime_type(response.headers_mut(), mime_type);

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
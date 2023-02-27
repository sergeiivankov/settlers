use http_body_util::Full;
use hyper::{
  body::Incoming, header::{ CONTENT_TYPE, HeaderValue }, Response, Request, StatusCode
};
use log::debug;
use std::{ io::{ Error, ErrorKind }, path::{ Component, Path, PathBuf } };
use crate::settings::SETTINGS;
use super::helpers::{ MIME_TYPES, HttpResponse, PreBuiltHeader, header_value, status_response };

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
use lazy_static::lazy_static;
#[cfg(feature = "public_resources_caching")]
use log::error;
#[cfg(feature = "public_resources_caching")]
use sha1::{ Sha1, Digest };
#[cfg(feature = "public_resources_caching")]
use std::{ collections::HashMap, fs::read };
#[cfg(feature = "public_resources_caching")]
use tokio::sync::Mutex;
#[cfg(feature = "public_resources_caching")]
use walkdir::WalkDir;
#[cfg(feature = "public_resources_caching")]
use crate::helpers::exit_with_error;

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

      let encoded_hash = encode(hash);
      let etag = HeaderValue::from_str(&format!("\"{}\"", encoded_hash)).unwrap_or_else(|_| {
        exit_with_error(format!("Create \"Etag\" header value error: {}", encoded_hash))
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

#[cfg(feature = "public_resources_caching")]
async fn get_response_data(
path: String, mime_type: HeaderValue, req: Request<Incoming>
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
      headers.insert(CONTENT_TYPE, mime_type);
      headers.insert(ETAG, hash.clone());

      Ok(response)
    },
    None => Err(Error::new(ErrorKind::NotFound, ""))
  }
}

#[cfg(not(feature = "public_resources_caching"))]
async fn get_response_data(
  path: String, mime_type: HeaderValue, _: Request<Incoming>
) -> Result<HttpResponse, Error> {
  let full_path = format!("{}{}{}", SETTINGS.public_resources_path, MAIN_SEPARATOR, path);

  match read(full_path).await {
    Ok(content) => {
      let mut response = Response::new(Full::new(content.into()));
      response.headers_mut().insert(CONTENT_TYPE, mime_type);

      Ok(response)
    },
    Err(err) => Err(err)
  }
}

pub async fn serve(path: &str, req: Request<Incoming>) -> HttpResponse {
  let path = {
    let mut normalized_path = PathBuf::new();

    // Path analisis for special components exists
    for component in Path::new(path).components() {
      match component {
        Component::Prefix(_) | Component::CurDir | Component::RootDir | Component::ParentDir => {
          debug!("Found special path component {:?} in \"{}\"", component, path);
          return status_response(StatusCode::NOT_FOUND)
        },
        Component::Normal(c) => normalized_path.push(c)
      };
    }

    let path_str = match normalized_path.to_str() {
      Some(path_str) => path_str,
      None => {
        debug!("Convert path \"{}\" to str error", normalized_path.display());
        return status_response(StatusCode::INTERNAL_SERVER_ERROR)
      }
    };

    String::from(path_str)
  };

  let mime_type = {
    let ext = match path.rsplit_once('.') {
      Some(parts) => parts.1,
      None => ""
    };

    match MIME_TYPES.get(ext) {
      Some(mime_type) => mime_type.clone(),
      None => header_value(PreBuiltHeader::AppOctetStream)
    }
  };

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
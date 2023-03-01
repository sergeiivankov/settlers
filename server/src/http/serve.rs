use http_body_util::Full;
use hyper::{ body::Incoming, header::{ CONTENT_TYPE, HeaderValue }, Response, Request, StatusCode };
use log::debug;
use std::{ clone::Clone, path::{ Component, Path, PathBuf } };
use crate::settings::SETTINGS;
use super::helpers::{ MIME_TYPES, HttpResponse, PreBuiltHeader, header_value, status_response };

#[cfg(not(feature = "public_resources_caching"))]
use log::{ Level, log };
#[cfg(not(feature = "public_resources_caching"))]
use std::{ io::ErrorKind, path::MAIN_SEPARATOR };
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
pub struct ResourceCache {
  mime_type: HeaderValue,
  etag: HeaderValue,
  body: Full<Bytes>
}

#[cfg(feature = "public_resources_caching")]
lazy_static! {
  // Read all public resources files to cache on server start
  // Values in HashMap is strust with mime type HeaderValue, content hash ETAG HeaderValue
  // and ready to return response body
  pub static ref PUBLIC_RESOURCES_CACHE: Mutex<HashMap<String, ResourceCache>> = {
    let mut paths = Vec::new();

    for entry_result in WalkDir::new(&SETTINGS.public_resources_path) {
      let entry = entry_result.unwrap_or_else(|err| {
        exit_with_error(&format!("Walk entry error: {err}"))
      });

      let path = entry.path().to_owned();
      if path.is_file() {
        paths.push(path);
      }
    }

    let mut cache = HashMap::with_capacity(paths.len());
    let mut hasher = Sha1::new();

    for path in &paths {
      let path_str = path.to_str().unwrap_or_else(|| {
        exit_with_error(&format!("Convert path \"{}\" to str error", path.display()))
      });

      let content = read(path).unwrap_or_else(|err| {
        exit_with_error(&format!("Read file \"{}\" error: {err}", path.display()))
      });

      hasher.update(&content);
      let hash = hasher.finalize_reset();

      let etag = format!("\"{}\"", encode(hash));
      let etag_value = HeaderValue::from_str(&etag).unwrap_or_else(|_| {
        exit_with_error(&format!("Create \"Etag\" header value for \"{path_str}\" error: {etag}"))
      });

      // Cut off path to public resources directory part from full public resource path
      let key = String::from(&path_str[(SETTINGS.public_resources_path.len() + 1)..]);

      cache.insert(key, ResourceCache {
        mime_type: get_mime_type(path_str),
        etag: etag_value,
        body: Full::new(content.into())
      });
    }

    Mutex::new(cache)
  };
}

fn get_mime_type(path: &str) -> HeaderValue {
  let ext = path.rsplit_once('.').map_or("", |parts| parts.1);

  MIME_TYPES.get(ext).map_or_else(
    || header_value(PreBuiltHeader::ApplicationOctetStream),
    Clone::clone
  )
}

#[cfg(feature = "public_resources_caching")]
async fn get_response_data(path: String, req: Request<Incoming>) -> HttpResponse {
  let cache = PUBLIC_RESOURCES_CACHE.lock().await;

  if let Some(resource_cache) = cache.get(&path) {
    if let Some(client_hash) = req.headers().get(IF_NONE_MATCH) {
      if client_hash == resource_cache.etag {
        let mut response = Response::new(Full::new(Bytes::new()));
        *response.status_mut() = StatusCode::NOT_MODIFIED;
        return response
      }
    }

    let mut response = Response::new(resource_cache.body.clone());

    let headers = response.headers_mut();
    headers.insert(CONTENT_TYPE, resource_cache.mime_type.clone());
    headers.insert(ETAG, resource_cache.etag.clone());

    return response
  }

  status_response(StatusCode::NOT_FOUND)
}

#[cfg(not(feature = "public_resources_caching"))]
async fn get_response_data(path: String, _: Request<Incoming>) -> HttpResponse {
  let full_path = format!("{}{MAIN_SEPARATOR}{path}", SETTINGS.public_resources_path);

  match read(&full_path).await {
    Ok(content) => {
      let mut response = Response::new(Full::new(content.into()));
      response.headers_mut().insert(CONTENT_TYPE, get_mime_type(&path));

      response
    },
    Err(err) => {
      let log_level = match err.kind() {
        ErrorKind::NotFound => Level::Debug,
        _ => Level::Warn
      };
      log!(log_level, "Read file \"{full_path}\" error: {err}");

      status_response(StatusCode::NOT_FOUND)
    }
  }
}

pub async fn serve(path: &str, req: Request<Incoming>) -> HttpResponse {
  let path = {
    let mut normalized_path = PathBuf::new();

    // Path analisis for special components exists
    for component in Path::new(path).components() {
      if let Component::Normal(c) = component {
        normalized_path.push(c);
      } else {
        debug!("Found special path component {component:?} in \"{path}\"");
        return status_response(StatusCode::NOT_FOUND)
      }
    }

    let Some(path_str) = normalized_path.to_str() else {
      debug!("Convert path \"{}\" to str error", normalized_path.display());
      return status_response(StatusCode::INTERNAL_SERVER_ERROR)
    };

    String::from(path_str)
  };

  get_response_data(path, req).await
}
use http_body_util::Full;
use hyper::{ body::Incoming, header::{ CONTENT_TYPE, HeaderValue }, Response, Request, StatusCode };
use log::debug;
use std::{ clone::Clone, path::{ Component, Path, PathBuf } };
use super::helpers::{ MIME_TYPES, HttpResponse, PreBuiltHeader, header_value, status_response };

// For default and packing
#[cfg(not(feature = "client_resources_caching"))]
use std::path::MAIN_SEPARATOR_STR as SEP;

// For default and caching
#[cfg(not(feature = "client_resources_packing"))]
use crate::settings::SETTINGS;

// For default only
#[cfg(not(any(feature = "client_resources_caching", feature = "client_resources_packing")))]
use log::{ Level, log };
#[cfg(not(any(feature = "client_resources_caching", feature = "client_resources_packing")))]
use std::io::ErrorKind;
#[cfg(not(any(feature = "client_resources_caching", feature = "client_resources_packing")))]
use tokio::fs::read;

#[cfg(feature = "client_resources_caching")]
use std::fs::read;
#[cfg(feature = "client_resources_caching")]
use walkdir::WalkDir;

#[cfg(feature = "client_resources_packing")]
use flate2::write::GzDecoder;
#[cfg(feature = "client_resources_packing")]
use std::io::Read;
#[cfg(feature = "client_resources_packing")]
use tar::{ Archive, EntryType };

// For caching and packing
#[cfg(any(feature = "client_resources_caching", feature = "client_resources_packing"))]
use bytes::Bytes;
#[cfg(any(feature = "client_resources_caching", feature = "client_resources_packing"))]
use flate2::{ write::GzEncoder, Compression };
#[cfg(any(feature = "client_resources_caching", feature = "client_resources_packing"))]
use hex::encode;
#[cfg(any(feature = "client_resources_caching", feature = "client_resources_packing"))]
use hyper::header::{ CONTENT_ENCODING, ETAG, IF_NONE_MATCH };
#[cfg(any(feature = "client_resources_caching", feature = "client_resources_packing"))]
use lazy_static::lazy_static;
#[cfg(any(feature = "client_resources_caching", feature = "client_resources_packing"))]
use sha1::{ Sha1, Digest };
#[cfg(any(feature = "client_resources_caching", feature = "client_resources_packing"))]
use std::{ collections::HashMap, io::Write };
#[cfg(any(feature = "client_resources_caching", feature = "client_resources_packing"))]
use tokio::sync::Mutex;
#[cfg(any(feature = "client_resources_caching", feature = "client_resources_packing"))]
use crate::helpers::exit_with_error;

#[cfg(any(feature = "client_resources_caching", feature = "client_resources_packing"))]
const GZIP_BLACKLIST: &[&str] = &["woff2"];

#[cfg(any(feature = "client_resources_caching", feature = "client_resources_packing"))]
pub struct ResourceCache {
  mime_type: HeaderValue,
  etag: HeaderValue,
  is_gzipped: bool,
  body: Full<Bytes>
}

#[cfg(feature = "client_resources_caching")]
lazy_static! {
  // Read all client resources files to memory on server start
  // Values in HashMap is strust with mime type HeaderValue, content hash ETAG HeaderValue
  // and ready to return response body
  pub static ref CLIENT_RESOURCES: Mutex<HashMap<String, ResourceCache>> = {
    let mut paths = Vec::new();

    for entry_result in WalkDir::new(&SETTINGS.client_resources_path) {
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

      let mut content = read(path).unwrap_or_else(|err| {
        exit_with_error(&format!("Read file \"{}\" error: {err}", path.display()))
      });

      hasher.update(&content);
      let hash = hasher.finalize_reset();

      let etag = format!("\"{}\"", encode(hash));
      let etag_value = HeaderValue::from_str(&etag).unwrap_or_else(|_| {
        exit_with_error(&format!("Create \"Etag\" header value for \"{path_str}\" error: {etag}"))
      });

      let is_gzipped = if GZIP_BLACKLIST.contains(&get_ext(path_str)) { false } else {
        let mut encoder = GzEncoder::new(Vec::new(), Compression::best());
        encoder.write_all(&content).unwrap_or_else(|err| {
          exit_with_error(&format!("Gzip encoder write for \"{path_str}\" error: {err}"))
        });
        content = encoder.finish().unwrap_or_else(|err| {
          exit_with_error(&format!("Gzip encoder finish for \"{path_str}\" error: {err}"))
        });

        true
      };

      // Cut off path to public resources directory part from full public resource path
      let key = String::from(&path_str[(SETTINGS.client_resources_path.len() + 1)..]);

      cache.insert(key, ResourceCache {
        mime_type: get_mime_type(path_str),
        etag: etag_value,
        is_gzipped,
        body: Full::new(content.into())
      });
    }

    Mutex::new(cache)
  };
}

#[cfg(feature = "client_resources_packing")]
lazy_static! {
  // Unpack all client resources files from included to binary archive to memory on server start
  // Values in HashMap is strust with mime type HeaderValue, content hash ETAG HeaderValue
  // and ready to return response body
  pub static ref CLIENT_RESOURCES: Mutex<HashMap<String, ResourceCache>> = {
    let mut decoder = GzDecoder::new(Vec::new());
    decoder.write_all(include_bytes!(concat!(env!("OUT_DIR"), "/dist.tar.gz")))
      .unwrap_or_else(|err| {
        exit_with_error(&format!("Write gzip decoder error: {err}"))
      });
    let content = decoder.finish().unwrap_or_else(|err| {
      exit_with_error(&format!("Finish gzip decoder error: {err}"))
    });

    let mut entries = Vec::new();

    for entry_result in Archive::new(&content[..]).entries().unwrap() {
      let mut entry = entry_result.unwrap_or_else(|err| {
        exit_with_error(&format!("Archive entry error: {err}"))
      });

      if entry.header().entry_type() != EntryType::Regular {
        continue
      }

      let path = entry.path().unwrap_or_else(|err| {
        exit_with_error(&format!("Get entry path error: {err}"))
      }).into_owned();

      let path_str = path.to_str().unwrap_or_else(|| {
        exit_with_error(&format!("Convert path \"{}\" to str error", path.display()))
      });

      // Client resource will not be more than u32::MAX, so truncation is impossible
      #[allow(clippy::cast_possible_truncation)]
      let mut content = vec![0; entry.size() as usize];
      entry.read_exact(&mut content).unwrap_or_else(|err| {
        exit_with_error(&format!("Read entry \"{}\" error: {err}", path.display()))
      });

      entries.push((path_str.to_string(), content));
    }

    let mut cache = HashMap::with_capacity(entries.len());
    let mut hasher = Sha1::new();

    for (path_string, mut content) in entries {
      hasher.update(&content);
      let hash = hasher.finalize_reset();

      let etag = format!("\"{}\"", encode(hash));
      let etag_value = HeaderValue::from_str(&etag).unwrap_or_else(|_| {
        exit_with_error(&format!("Create \"Etag\" header value for \"{path_string}\" error: {etag}"))
      });

      let is_gzipped = if GZIP_BLACKLIST.contains(&get_ext(&path_string)) { false } else {
        let mut encoder = GzEncoder::new(Vec::new(), Compression::best());
        encoder.write_all(&content).unwrap_or_else(|err| {
          exit_with_error(&format!("Gzip encoder write for \"{path_string}\" error: {err}"))
        });
        content = encoder.finish().unwrap_or_else(|err| {
          exit_with_error(&format!("Gzip encoder finish for \"{path_string}\" error: {err}"))
        });

        true
      };

      // Archive path use '/' as separator, so need replace it by current platform separator
      let key = path_string.clone().replace('/', SEP);

      cache.insert(key, ResourceCache {
        mime_type: get_mime_type(&path_string),
        etag: etag_value,
        is_gzipped,
        body: Full::new(content.into())
      });
    }

    Mutex::new(cache)
  };
}

fn get_ext(path: &str) -> &str {
  path.rsplit_once('.').map_or("", |parts| parts.1)
}

fn get_mime_type(path: &str) -> HeaderValue {
  MIME_TYPES.get(get_ext(path)).map_or_else(
    || header_value(PreBuiltHeader::ApplicationOctetStream),
    Clone::clone
  )
}

#[cfg(any(feature = "client_resources_caching", feature = "client_resources_packing"))]
async fn get_response_data(path: String, req: Request<Incoming>) -> HttpResponse {
  let cache = CLIENT_RESOURCES.lock().await;

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
    if resource_cache.is_gzipped {
      headers.insert(CONTENT_ENCODING, header_value(PreBuiltHeader::Gzip));
    }

    return response
  }

  status_response(StatusCode::NOT_FOUND)
}

#[cfg(not(any(feature = "client_resources_caching", feature = "client_resources_packing")))]
async fn get_response_data(path: String, _: Request<Incoming>) -> HttpResponse {
  let full_path = format!("{}{SEP}{path}", SETTINGS.client_resources_path);

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
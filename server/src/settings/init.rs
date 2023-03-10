use config::{ builder::DefaultState, Config, ConfigBuilder, Environment, File };
use dirs::config_dir;
use lazy_static::{ lazy_static, initialize };
use log::{ debug, info };
use serde_path_to_error::deserialize;
use std::{
  env::current_dir, fs::metadata, io::{ Error, ErrorKind },
  path::{ MAIN_SEPARATOR as SEP, Path, PathBuf }
};
use crate::helpers::exit_with_error;
use super::structs::Settings;

lazy_static! {
  static ref CURRENT_PATH: PathBuf = current_dir().unwrap_or_else(|err| {
    exit_with_error(&format!("Get current path error: {err}"))
  });
}

fn check_config_path(path: PathBuf) -> Option<PathBuf> {
  let err = match metadata(&path) {
    Ok(metadata) => {
      if metadata.is_file() {
        return Some(path)
      }

      Error::new(ErrorKind::InvalidInput, "path is directory")
    },
    Err(err) => {
      if err.kind() == ErrorKind::NotFound {
        return None
      }

      err
    }
  };

  exit_with_error(&format!("Read config file \"{}\" error: {err}", path.display()))
}

fn search_config_current_recurse(directory: &Path) -> Option<PathBuf> {
  let path = directory.join("settlers.toml");

  let checked_config_path = check_config_path(path);
  if checked_config_path.is_some() { return checked_config_path }

  directory.parent().and_then(search_config_current_recurse)
}

fn try_add_file_source(
  builder: ConfigBuilder<DefaultState>, path: PathBuf, need_check: bool
) -> ConfigBuilder<DefaultState> {
  let path = if need_check {
    if let Some(path) = check_config_path(path.clone()) {
      path
    } else {
      debug!("Config file \"{}\" not found", path.display());
      return builder
    }
  } else {
    path
  };

  let path_str = path.to_str().unwrap_or_else(|| {
    exit_with_error(&format!("Convert path \"{}\" to str error", path.display()))
  });

  info!("Config source added \"{path_str}\"");
  builder.add_source(File::with_name(path_str))
}

#[cfg(not(feature = "client_resources_packing"))]
fn prepare_check_path(path_string: &String, must_be_file: bool) -> String {
  let mut path = PathBuf::from(&path_string);

  path = if path.is_relative() { CURRENT_PATH.join(path_string) } else { path };

  // Return NotFound error kind for non-existent file,
  // so canonicalize - it also a check for existence
  path = path.canonicalize().unwrap_or_else(|_| {
    exit_with_error(&format!("Path \"{}\" not exists", path.display()))
  });

  if path.is_file() != must_be_file {
    exit_with_error(&
      if must_be_file { format!("Path \"{}\" must point to file", path.display()) }
      else { format!("Path \"{}\" must point to directory", path.display()) }
    )
  }

  let path_str = path.to_str().unwrap_or_else(|| {
    exit_with_error(&format!("Convert path \"{}\" to str error", path.display()))
  });

  String::from(path_str)
}

fn default(settings: &mut Settings) {
  settings.log = settings.log.clone().or_else(|| Some("error".to_string()));

  settings.database.min_connections = settings.database.min_connections.or(Some(1));
  settings.database.max_connections = settings.database.max_connections.or(Some(32));
  settings.database.connect_timeout = settings.database.connect_timeout.or(Some(10));
  settings.database.acquire_timeout = settings.database.acquire_timeout.or(Some(10));
  settings.database.idle_timeout = settings.database.idle_timeout.or(Some(10));
  settings.database.max_lifetime = settings.database.max_lifetime.or(Some(10));
}

#[cfg(not(feature = "client_resources_packing"))]
fn check(settings: &mut Settings) {
  settings.client_resources_path = prepare_check_path(&settings.client_resources_path, false);

  #[cfg(feature = "secure_server")]
  (settings.secure_server.cert_path = prepare_check_path(&settings.secure_server.cert_path, true));
  #[cfg(feature = "secure_server")]
  (settings.secure_server.key_path = prepare_check_path(&settings.secure_server.key_path, true));
}

pub fn init() -> Settings {
  // Need to check for lazy static current path initialize errors before start settings parsing
  initialize(&CURRENT_PATH);

  let mut builder = Config::builder();

  #[cfg(target_os = "linux")]
  (builder = try_add_file_source(builder, PathBuf::from("/etc/settlers/settlers.toml"), true));

  if let Some(directory) = config_dir() {
    let path = directory.join(format!("settlers{SEP}settlers.toml"));
    builder = try_add_file_source(builder, path, true);
  }

  // In Some variant local variable `builder` rewrites,
  // so Option::map_or_else with closures is not work
  #[allow(clippy::option_if_let_else)]
  if let Some(path) = search_config_current_recurse(&CURRENT_PATH) {
    builder = try_add_file_source(builder, path, false);
  } else {
    debug!("Config file not found in current directory tree");
  }

  builder = builder.add_source(Environment::with_prefix("settlers"));

  let config = builder.build().unwrap_or_else(|err| {
    exit_with_error(&format!("Build config error: {err}"))
  });

  let mut settings: Settings = deserialize(config).unwrap_or_else(|err| {
    let path = err.path().to_string();
    let key_part = if path == "." { String::new() } else { format!(" key \"{path}\"") };
    exit_with_error(&format!("Config{key_part} error: {}", err.inner()))
  });

  default(&mut settings);

  #[cfg(not(feature = "client_resources_packing"))]
  check(&mut settings);

  settings
}
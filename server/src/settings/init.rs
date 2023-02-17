use config::{ builder::DefaultState, Config, ConfigBuilder, Environment, File };
use dirs::config_dir;
use log::{ debug, info };
use std::{ fs::metadata, io::{ Error, ErrorKind }, path::PathBuf };
use crate::helpers::{ CURRENT_PATH, exit_with_error, prepare_check_path };
use super::structs::Settings;

fn check_config_path(path: PathBuf) -> Option<PathBuf> {
  let err = match metadata(&path) {
    Ok(metadata) => {
      if metadata.is_file() {
        return Some(path)
      } else {
        Error::new(ErrorKind::InvalidInput, "path is directory")
      }
    },
    Err(err) => match err.kind() {
      ErrorKind::NotFound => return None,
      _ => err
    }
  };

  exit_with_error(format!("Read config file \"{}\" error: {}", path.display(), err))
}

fn search_config_current_recurse(directory: &PathBuf) -> Option<PathBuf> {
  let path = directory.join("settlers.toml");

  match check_config_path(path) {
    Some(path) => return Some(path),
    None => {}
  }

  match directory.parent() {
    Some(parent) => search_config_current_recurse(&parent.to_path_buf()),
    None => None
  }
}

fn try_add_file_source(
  builder: ConfigBuilder<DefaultState>, path: PathBuf, need_check: bool
) -> ConfigBuilder<DefaultState> {
  let path = if need_check {
    match check_config_path(path.clone()) {
      Some(path) => path,
      None => {
        debug!("Config file \"{}\" not found", path.display());
        return builder
      }
    }
  } else {
    path
  };

  let path_str = match path.to_str() {
    Some(path_str) => path_str,
    None => exit_with_error(format!("Convert path \"{}\" to str error", path.display()))
  };

  info!("Config source added \"{}\"", path_str);
  builder.add_source(File::with_name(path_str))
}

fn check(settings: &mut Settings) {
  settings.public_resources_path = prepare_check_path(&settings.public_resources_path, false);

  #[cfg(feature = "secure_server")]
  {
    settings.secure_server.cert_path = prepare_check_path(&settings.secure_server.cert_path, true);
    settings.secure_server.key_path = prepare_check_path(&settings.secure_server.key_path, true);
  }
}

pub fn init() -> Settings {
  let mut builder = Config::builder();

  #[cfg(target_os = "linux")]
  (builder = try_add_file_source(builder, PathBuf::from("/etc/settlers/settlers.toml"), true));

  match config_dir() {
    Some(directory) => {
      let path = directory.join("settlers/settlers.toml");
      builder = try_add_file_source(builder, path, true);
    },
    None => {}
  }

  match search_config_current_recurse(&*CURRENT_PATH) {
    Some(path) => builder = try_add_file_source(builder, path, false),
    None => debug!("Config file not found in current directory tree")
  }

  builder = builder.add_source(Environment::with_prefix("settlers"));

  let config = match builder.build() {
    Ok(config) => config,
    Err(err) => exit_with_error(format!("Build config error: {}", err))
  };

  let mut settings: Settings = match config.try_deserialize() {
    Ok(settings) => settings,
    Err(err) => exit_with_error(format!("Deserialize config error: {:#?}", err))
  };

  check(&mut settings);

  settings
}
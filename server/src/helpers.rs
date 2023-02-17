use lazy_static::lazy_static;
use std::{ env::current_dir, path::PathBuf, process::exit };

lazy_static! {
  pub static ref CURRENT_PATH: PathBuf = current_dir()
    .unwrap_or_else(|err| exit_with_error(format!("Get current path error: {}", err)));
}

pub fn exit_with_error(error: String) -> ! {
  eprintln!("{}", error);
  exit(1)
}

pub fn prepare_check_path(path_string: &String, must_be_file: bool) -> String {
  let mut path = PathBuf::from(&path_string);

  path = if path.is_relative() {
    CURRENT_PATH.clone().join(path_string)
  } else {
    path
  };

  // Return NotFound error kind for non-existent file,
  // so canonicalize - it also a check for existence
  path = path.canonicalize().unwrap_or_else(|_| {
    exit_with_error(format!("Path \"{}\" not exists", path.display()))
  });

  if path.is_file() != must_be_file {
    if must_be_file {
      exit_with_error(format!("Path \"{}\" must point to file", path.display()))
    } else {
      exit_with_error(format!("Path \"{}\" must point to directory", path.display()))
    }
  }

  let path_str = path.to_str().unwrap_or_else(|| {
    exit_with_error(format!("Convert path \"{}\" to str error", path.display()))
  });

  String::from(path_str)
}
use lazy_static::lazy_static;
use std::{ env::current_dir, path::{ Path, PathBuf }, process::exit };

lazy_static! {
  pub static ref CURRENT_PATH: PathBuf = current_dir()
    .unwrap_or_else(|err| exit_with_error(format!("Get current path error: {}", err)));
}

pub fn exit_with_error(error: String) -> ! {
  eprintln!("{}", error);
  exit(1)
}

pub fn prepare_check_path(path_string: &String, must_be_file: bool) -> String {
  let mut path = Path::new(&path_string);

  let mut path_absolute: PathBuf;
  if path.is_relative() {
    path_absolute = CURRENT_PATH.clone();
    path_absolute.push(path_string);
    path = Path::new(&path_absolute);
  }

  // In my cases, always return NotFound error kind, so canonicalize - it also a check for existence
  let path = path.canonicalize().unwrap_or_else(|_| {
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
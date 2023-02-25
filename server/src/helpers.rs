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
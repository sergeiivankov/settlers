use lazy_static::lazy_static;
use std::{ env::current_dir, path::PathBuf, process::exit };

lazy_static! {
  pub static ref CURRENT_PATH: PathBuf = current_dir()
    .unwrap_or_else(|err| exit_with_error(format!("Get current path error: {err}")));
}

// Function calls with error and terminates the current process, so error message not used later
#[allow(clippy::needless_pass_by_value)]
pub fn exit_with_error(error: String) -> ! {
  eprintln!("{error}");
  exit(1)
}
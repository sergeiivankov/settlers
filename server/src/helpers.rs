use std::process::exit;

pub fn exit_with_error(error: &str) -> ! {
  eprintln!("{error}");
  exit(1)
}
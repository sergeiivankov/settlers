use std::process::exit;

// Function calls with error and terminates the current process, so error message not used later
#[allow(clippy::needless_pass_by_value)]
pub fn exit_with_error(error: String) -> ! {
  eprintln!("{error}");
  exit(1)
}
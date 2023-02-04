use std::{ env::{ VarError, var }, process::exit };

pub fn get_env(name: &str) -> String {
  match var(name) {
    Ok(value) => value,
    Err(VarError::NotPresent) => {
      exit_with_error(format!("Required environment value \"{}\" not present", name))
    }
    Err(VarError::NotUnicode(os_string)) => {
      exit_with_error(format!(
        "Read environment value \"{}\" error, \"{:?}\" is not contain valid unicode data",
        name, os_string
      ))
    }
  }
}

pub fn exit_with_error(error: String) -> ! {
  eprintln!("{}", error);
  exit(1)
}
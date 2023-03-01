use pb_rs::{ types::FileDescriptor, ConfigBuilder };
use std::{ env::var, fs::{ create_dir_all, remove_dir_all }, path::{ MAIN_SEPARATOR, PathBuf } };
use walkdir::WalkDir;

fn main() {
  println!("cargo:rerun-if-changed=../protos");

  let cargo_manifest_dir_path = PathBuf::from(&var("CARGO_MANIFEST_DIR").unwrap());

  let out_dir = cargo_manifest_dir_path.join(format!("src{MAIN_SEPARATOR}protos"));
  let in_dir = cargo_manifest_dir_path.parent().unwrap().join("protos");

  let mut protos = Vec::new();
  for entry_result in WalkDir::new(&in_dir) {
    let path = entry_result.unwrap().into_path();
    if path.is_file() {
      protos.push(path);
    }
  }

  if out_dir.exists() {
    remove_dir_all(&out_dir).unwrap();
  }
  create_dir_all(&out_dir).unwrap();

  let config_builder = ConfigBuilder::new(&protos, None, Some(&out_dir), &[in_dir])
    .unwrap()
    .dont_use_cow(true)
    .headers(false);

  FileDescriptor::run(&config_builder.build()).unwrap();
}
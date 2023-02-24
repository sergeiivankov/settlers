use pb_rs::{ types::FileDescriptor, ConfigBuilder };
use std::{ env::var, fs::{ create_dir_all, remove_dir_all }, path::Path };
use walkdir::WalkDir;

fn main() {
  println!("cargo:rerun-if-changed=../protos");

  let out_dir = Path::new(&var("OUT_DIR").unwrap()).join("protos");
  let in_dir = Path::new(&var("CARGO_MANIFEST_DIR").unwrap()).parent().unwrap().join("protos");

  let mut protos = Vec::new();
  for entry in WalkDir::new(&in_dir) {
    let path = entry.unwrap().into_path();
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
    .dont_use_cow(true);

  FileDescriptor::run(&config_builder.build()).unwrap();
}
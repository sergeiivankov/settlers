use pb_rs::{ types::FileDescriptor, ConfigBuilder };
use std::{
  env::var, fs::{ create_dir_all, remove_dir_all }, path::{ MAIN_SEPARATOR as SEP, PathBuf }
};
use walkdir::WalkDir;

#[cfg(feature = "client_resources_packing")]
use flate2::{ write::GzEncoder, Compression };
#[cfg(feature = "client_resources_packing")]
use std::{ fs::File, io::Write };
#[cfg(feature = "client_resources_packing")]
use tar::Builder;

#[cfg(any(feature = "client_resources_caching", feature = "client_resources_packing"))]
use std::process::Command;

fn main() {
  let cargo_manifest_dir_path = PathBuf::from(&var("CARGO_MANIFEST_DIR").unwrap());

  // Client compilation
  #[cfg(any(feature = "client_resources_caching", feature = "client_resources_packing"))]
  {
    println!("cargo:rerun-if-changed=../client");

    let output = Command::new("cmd")
      .arg("/c npm run build")
      .current_dir("../client")
      .output()
      .unwrap();

    if !output.status.success() {
      panic!(
        "{}{}",
        String::from_utf8(output.stdout).unwrap(),
        String::from_utf8(output.stderr).unwrap()
      )
    }
  }

  // Builded client packing
  #[cfg(feature = "client_resources_packing")]
  {
    let mut tar = Builder::new(Vec::new());
    tar.append_dir_all(
      "", cargo_manifest_dir_path.parent().unwrap().join(format!("client{SEP}dist"))
    ).unwrap();
    tar.finish().unwrap();

    let content = tar.into_inner().unwrap();

    let file = File::create(PathBuf::from(&var("OUT_DIR").unwrap()).join("dist.tar.gz")).unwrap();

    let mut encoder = GzEncoder::new(file, Compression::best());
    encoder.write_all(&content).unwrap();
    encoder.finish().unwrap();
  }

  // Protocol buffer structures compilation
  {
    println!("cargo:rerun-if-changed=../protos");

    let out_dir_path = cargo_manifest_dir_path.join(format!("src{SEP}protos"));
    let in_dir_path = cargo_manifest_dir_path.parent().unwrap().join("protos");

    let mut protos = Vec::new();
    for entry_result in WalkDir::new(&in_dir_path) {
      let path = entry_result.unwrap().into_path();
      if path.is_file() {
        protos.push(path);
      }
    }

    if out_dir_path.exists() {
      remove_dir_all(&out_dir_path).unwrap();
    }
    create_dir_all(&out_dir_path).unwrap();

    let config_builder = ConfigBuilder::new(&protos, None, Some(&out_dir_path), &[in_dir_path])
      .unwrap()
      .dont_use_cow(true)
      .headers(false);

    FileDescriptor::run(&config_builder.build()).unwrap();
  }
}
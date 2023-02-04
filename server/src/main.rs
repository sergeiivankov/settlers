use dotenv::dotenv;
use env_logger::{ Builder as EnvLoggerBuilder, Env };
use log::{ LevelFilter, info };
use tokio::runtime::Builder as RuntimeBuilder;
use crate::{ helpers::exit_with_error, http::start };

fn main() {
  // Easy way to disable rustls crate self-signed certificate client error
  // If you need logs from rustls::conn module, delete filter or use .format env_logger::Builder
  // method to filter exactly this error according to its content
  EnvLoggerBuilder::from_env(Env::default().default_filter_or("error"))
    .filter_module("rustls::conn", LevelFilter::Off)
    .init();

  match dotenv() {
    Ok(path) => info!("Environment variables loaded from \"{}\"", path.as_path().display()),
    Err(_) => info!("File with environment variables not found")
  };

  let runtime = RuntimeBuilder::new_multi_thread().enable_io().enable_time().build()
    .unwrap_or_else(|err| exit_with_error(format!("Create tokio runtime error: {}", err)));

  runtime.block_on(async {
    start().await
  });
}

mod helpers;
mod http;
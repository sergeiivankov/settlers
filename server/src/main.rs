use dotenv::dotenv;
use env_logger::{ Builder as EnvLoggerBuilder, Env };
use log::{ LevelFilter, debug, error, info };
use tokio::{
  runtime::Builder as RuntimeBuilder, signal::ctrl_c, sync::oneshot::channel, join, spawn
};
use crate::{
  communicator::Communicator, helpers::exit_with_error, http::start, intermedium::Intermedium
};

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

  let runtime = RuntimeBuilder::new_multi_thread().enable_io().build()
    .unwrap_or_else(|err| exit_with_error(format!("Create tokio runtime error: {}", err)));

  runtime.block_on(async {
    let (intermedium_stop_sender, intermedium_stop_receiver) = channel::<()>();
    let (http_stop_sender, http_stop_receiver) = channel::<()>();

    let (communicator, receiver) = Communicator::new();
    let mut intermedium = Intermedium::new(communicator.clone(), receiver);

    let intermedium_handle = spawn(async move {
      intermedium.run(intermedium_stop_receiver).await
    });

    let http_handle = spawn(start(communicator, http_stop_receiver));

    let stop_handle = spawn(async move {
      match ctrl_c().await {
        Ok(_) => {},
        Err(err) => exit_with_error(format!("Receive Ctrl-C signal error: {}", err))
      };

      debug!("Received Ctrl-C signal");

      match intermedium_stop_sender.send(()) {
        Ok(_) => {},
        Err(_) => exit_with_error(String::from("Send intermedium stop signal error"))
      };

      match http_stop_sender.send(()) {
        Ok(_) => {},
        Err(_) => exit_with_error(String::from("Send http stop signal error"))
      };
    });

    let (intermedium_join_result, http_join_result, stop_join_result) = join!(
      intermedium_handle, http_handle, stop_handle
    );

    match intermedium_join_result {
      Ok(_) => {},
      Err(err) => error!("Join intermedium task error: {}", err)
    }
    match http_join_result {
      Ok(_) => {},
      Err(err) => error!("Join http task error: {}", err)
    }
    match stop_join_result {
      Ok(_) => {},
      Err(err) => error!("Join stop task error: {}", err)
    }
  });
}

mod communicator;
mod helpers;
mod http;
mod intermedium;
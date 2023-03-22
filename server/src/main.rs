#![deny(clippy::all)]
#![deny(clippy::pedantic)]
// TODO: write documentation and enable next lints category
//#![deny(clippy::restriction)]
#![deny(clippy::nursery)]
#![deny(clippy::cargo)]

// Project will not published on crates.io, so no need for fields "keywords" and "categories"
#![allow(clippy::cargo_common_metadata)]

// TODO: design project repository

#[cfg(all(feature = "client_resources_caching", feature = "client_resources_packing"))]
compile_error!("Features `client_resources_...` cannot be enabled at the same time");

#[cfg(not(any(
  feature = "db_mysql",
  feature = "db_postgres",
  feature = "db_sqlite"
)))]
compile_error!("Using one of `db_...` features is required");

mod communicator;
mod db;
mod helpers;
mod http;
mod intermedium;
mod protos {
  // Disable lints for automatically generated files
  #![allow(unused_imports)]
  #![allow(clippy::bool_comparison)]
  #![allow(clippy::cast_lossless)]
  #![allow(clippy::deref_addrof)]
  #![allow(clippy::explicit_auto_deref)]
  #![allow(clippy::identity_op)]
  #![allow(clippy::needless_borrow)]
  #![allow(clippy::wildcard_imports)]
  include!(concat!(env!("CARGO_MANIFEST_DIR"), "/src/protos/mod.rs"));
}
mod settings;

use dotenv::dotenv;
use env_logger::Builder as EnvLoggerBuilder;
use lazy_static::initialize;
use log::{ Level, LevelFilter, debug, error };
use sea_orm::{ ConnectOptions, Database };
use sea_orm_migration::MigratorTrait;
use std::{ io::Write, time::Duration };
use tokio::{
  runtime::Builder as RuntimeBuilder, signal::ctrl_c, sync::oneshot::channel, join, spawn
};
use crate::{
  communicator::Communicator, db::Migrator, helpers::exit_with_error,
  http::start, intermedium::Intermedium, settings::SETTINGS
};

fn main() {
  // Before initialize settings and EnvLogger try read .env file
  // It may contain RUST_LOG or SETTLERS_* variables
  dotenv().ok();

  // Need to check for lazy static settings initialize errors before server start
  initialize(&SETTINGS);

  // For logging initialization "log" config value usage
  let mut env_logger_builder = EnvLoggerBuilder::new();
  env_logger_builder.parse_filters(SETTINGS.log.as_ref().unwrap());
  env_logger_builder.format(|buf, record| {
    let level = match record.level() {
      Level::Debug => "DBG",
      Level::Error => "ERR",
      Level::Info => "INF",
      Level::Trace => "TRC",
      Level::Warn => "WRN"
    };

    writeln!(buf, "{} {level} {}", buf.timestamp_seconds(), record.args())
  });

  // Disable rustls crate logging by default (in particular, self-signed certificate client error)
  // TODO: if https://github.com/launchbadge/sqlx/pull/2371 will be accepted,
  //       rewrite to rustls crate feature "logging" using
  #[cfg(all(feature = "secure_server", not(feature = "rustls_logging")))]
  env_logger_builder.filter_module("rustls", LevelFilter::Off);

  env_logger_builder.init();

  let runtime = RuntimeBuilder::new_multi_thread().enable_io().enable_time().build()
    .unwrap_or_else(|err| {
      exit_with_error(&format!("Create tokio runtime error: {err}"))
    });

  runtime.block_on(async {
    let db_connect_options = ConnectOptions::new(SETTINGS.database.url.clone())
      .min_connections(SETTINGS.database.min_connections.unwrap())
      .max_connections(SETTINGS.database.max_connections.unwrap())
      .connect_timeout(Duration::from_secs(SETTINGS.database.connect_timeout.unwrap()))
      .acquire_timeout(Duration::from_secs(SETTINGS.database.acquire_timeout.unwrap()))
      .idle_timeout(Duration::from_secs(SETTINGS.database.idle_timeout.unwrap()))
      .max_lifetime(Duration::from_secs(SETTINGS.database.max_lifetime.unwrap()))
      // Set max logging level, it filtered by "log" config value inside "sqlx" crate
      // To control database logging, use "sqlx=level" in "log" config value
      .sqlx_logging_level(LevelFilter::Debug)
      .sqlx_logging(true)
      .clone();

    let db = Database::connect(db_connect_options).await.unwrap_or_else(|err| {
      exit_with_error(&format!("Database connect error: {err}"))
    });

    if let Err(err) = Migrator::up(&db, None).await {
      exit_with_error(&format!("Database migration error: {err}"))
    }

    let (intermedium_stop_sender, intermedium_stop_receiver) = channel::<()>();
    let (http_stop_sender, http_stop_receiver) = channel::<()>();

    let (communicator, receiver) = Communicator::new();
    let mut intermedium = Intermedium::new(communicator.clone(), receiver);

    let intermedium_handle = spawn(async move {
      intermedium.run(intermedium_stop_receiver).await;
    });

    let http_handle = spawn(start(communicator, http_stop_receiver));

    let stop_handle = spawn(async move {
      if let Err(err) = ctrl_c().await {
        exit_with_error(&format!("Receive Ctrl-C signal error: {err}"))
      }

      debug!("Received Ctrl-C signal");

      if intermedium_stop_sender.send(()).is_err() {
        exit_with_error("Send intermedium stop signal error")
      }

      if http_stop_sender.send(()).is_err() {
        exit_with_error("Send http stop signal error")
      }
    });

    let (intermedium_join_result, http_join_result, stop_join_result) = join!(
      intermedium_handle, http_handle, stop_handle
    );

    if let Err(err) = intermedium_join_result {
      error!("Join intermedium task error: {err}");
    }
    if let Err(err) = http_join_result {
      error!("Join http task error: {err}");
    }
    if let Err(err) = stop_join_result {
      error!("Join stop task error: {err}");
    }
  });
}
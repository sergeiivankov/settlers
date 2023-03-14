use serde::Deserialize;
use std::net::SocketAddr;

#[derive(Debug, Deserialize)]
pub struct Database {
  pub url: String,
  pub min_connections: Option<u32>,
  pub max_connections: Option<u32>,
  pub connect_timeout: Option<u64>,
  pub acquire_timeout: Option<u64>,
  pub idle_timeout: Option<u64>,
  pub max_lifetime: Option<u64>
}

#[cfg(feature = "secure_server")]
#[derive(Debug, Deserialize)]
pub struct SecureServer {
  pub cert_path: String,
  pub key_path: String
}

#[derive(Debug, Deserialize)]
pub struct Settings {
  pub log: Option<String>,
  pub bind_addr: SocketAddr,
  #[cfg(not(feature = "client_resources_packing"))]
  pub client_resources_path: String,
  pub database: Database,
  #[cfg(feature = "secure_server")]
  pub secure_server: SecureServer
}
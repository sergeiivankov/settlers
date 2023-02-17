use serde::Deserialize;
use std::net::SocketAddr;

#[derive(Debug, Deserialize)]
pub struct Database {
  pub url: String
}

#[cfg(feature = "secure_server")]
#[derive(Debug, Deserialize)]
pub struct SecureServer {
  pub cert_path: String,
  pub key_path: String
}

#[derive(Debug, Deserialize)]
pub struct Settings {
  pub bind_addr: SocketAddr,
  pub public_resources_path: String,
  pub database: Database,
  #[cfg(feature = "secure_server")]
  pub secure_server: SecureServer
}
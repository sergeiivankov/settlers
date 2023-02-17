mod init;
mod structs;

use lazy_static::lazy_static;
use self::{ init::init, structs::Settings };

lazy_static! {
  pub static ref SETTINGS: Settings = init();
}
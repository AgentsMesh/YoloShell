mod client;
mod constants;
mod error;
mod exchange;
mod local;

pub use client::Client;
pub use error::ClientError;
pub use local::socket_for_hub;
pub type LocalClient = Client<yolo_local_unix::LocalUnixStream>;

#[cfg(test)]
#[path = "../unit_tests/lib.rs"]
mod unit_tests;

mod client;
mod connector;
mod constants;
mod endpoint;
mod error;
mod exchange;
mod local;
mod remote_setup;
mod stream;

pub use client::Client;
pub use connector::EntryConnector;
pub use endpoint::HubEndpoint;
pub use error::ClientError;
pub use stream::{HubEntryIo, HubEntryStream};
pub type LocalClient = Client<yolo_local_unix::LocalUnixStream>;
pub type EndpointClient = Client<HubEntryStream>;

#[cfg(test)]
#[path = "../unit_tests/lib.rs"]
mod unit_tests;

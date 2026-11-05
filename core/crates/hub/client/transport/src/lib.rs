//! Authenticated public-client channels, independent of Hub federation authority.
mod accept;
mod client;
mod connect;
mod connection;
mod constants;
mod custody;
mod io;
mod pin;
mod server;
mod stream;
mod time;

pub use accept::accept;
pub use connect::connect;
pub use connection::{AcceptedConnection, ClientConnection};
pub use constants::{MAX_CLIENT_CHANNELS, MAX_QUEUED_CHANNELS, PUBLIC_CLIENT_USER};
pub use io::ClientIo;
pub use pin::HubPin;
pub use stream::{ClientStream, Duplex, IncomingChannel};

#[cfg(test)]
#[path = "../unit_tests/lib.rs"]
mod unit_tests;

//! Encrypted, mutually pinned Hub transport. Dial direction does not grant export rights.
mod admission;
mod client;
mod connect;
mod connection;
mod constants;
mod io;
mod io_custody;
mod pins;
mod server;
mod socket;
mod stream;

pub use admission::ChannelAdmissionError;
pub use connect::{accept, accept_io, connect, connect_io};
pub use connection::PeerConnection;
pub use constants::{MAX_PEER_CHANNELS, MAX_QUEUED_CHANNELS};
pub use io::PeerIo;
pub use pins::PeerPins;
pub use stream::{Duplex, PeerStream};

#[cfg(test)]
#[path = "../unit_tests/lib.rs"]
mod unit_tests;

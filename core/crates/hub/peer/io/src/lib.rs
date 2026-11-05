//! One peer byte stream plus the authority to close its transport.
mod custody;
mod io;

pub use io::{Duplex, PeerIo};

#[cfg(test)]
#[path = "../unit_tests/lib.rs"]
mod unit_tests;

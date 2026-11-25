//! Direct public Hub entry through one pinned SSH account connection.
mod connect;
mod connection;
mod constants;
mod endpoint;
mod error;
mod handler;
mod lifecycle;
mod profile;
mod socket;
mod validation;
mod wire;

pub use constants::{ENTRY_PROFILE_SCHEMA, MAX_PROFILE_BYTES};
pub use endpoint::SshEntry;
pub use error::ProfileError;
pub use profile::EntryProfile;

#[cfg(test)]
#[path = "../unit_tests/lib.rs"]
mod unit_tests;

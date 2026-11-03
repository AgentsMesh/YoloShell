//! Revocable authorization for already verified public-client SSH keys.
//! This component verifies configuration, not signatures or peer identity.
mod admission;
mod constants;
mod error;
mod registry;

pub use admission::ClientAdmission;
pub use error::RegistryError;
pub use registry::AccessRegistry;

#[cfg(test)]
#[path = "../unit_tests/lib.rs"]
mod unit_tests;

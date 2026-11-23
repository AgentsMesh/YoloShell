mod change;
mod constants;
mod record;
mod store;

pub use change::{RevisionConflict, RevisionExhausted};
pub use constants::CONFIGURATION_DIRECTORY;
pub use store::ConfigurationStore;

#[cfg(test)]
#[path = "../unit_tests/lib.rs"]
mod unit_tests;

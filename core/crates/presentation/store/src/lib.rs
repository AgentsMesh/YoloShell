//! Durable layout ownership and atomic revision changes, without terminal lifecycle authority.
mod constants;
mod error;
mod open;
mod record;
mod store;

pub use constants::{LAYOUT_FILE, MAX_DOCUMENTS};
pub use error::StoreError;
pub use store::LayoutStore;

#[cfg(test)]
#[path = "../unit_tests/lib.rs"]
mod unit_tests;

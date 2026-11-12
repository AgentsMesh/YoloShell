//! Single-owner durable public-client authorization, bound to its entry Hub.
//! Failed verification or writes poison this instance; no inferred recovery
//! permits an uncertain configuration to remain active.
mod constants;
mod open;
mod record;
mod store;

pub use constants::CLIENT_ACCESS_DIRECTORY;
pub use store::ClientAccessStore;

#[cfg(test)]
extern crate self as yolo_hub_client_access_store;

#[cfg(test)]
#[path = "../unit_tests/lib.rs"]
mod unit_tests;

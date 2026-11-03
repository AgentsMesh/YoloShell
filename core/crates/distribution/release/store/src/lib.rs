//! Durable local publication of a complete immutable runtime bundle.
mod checkpoint;
mod constants;
mod existing;
mod file;
mod file_seal;
mod inventory;
mod publication;
mod release;
mod store;

pub use constants::{
    CURRENT_FILE_NAME, EXECUTABLE_MODE, MANIFEST_MODE, SELECTION_SCHEMA_VERSION, STAGE_PREFIX,
};
pub use store::{InstalledBundle, ReleaseStore};
pub use yolo_distribution_publication_protocol::{Selection, SelectionFields};

#[cfg(test)]
extern crate self as yolo_distribution_release_store;
#[cfg(test)]
#[path = "../unit_tests/lib.rs"]
mod unit_tests;

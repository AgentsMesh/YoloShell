mod catalog;
mod catalog_edit;
mod client_location;
mod client_registry;
mod constants;
mod document;
mod location;
mod shared_layout;
mod store;

pub use catalog::{TabCatalogDocument, TabEntry, TabCatalog};
pub use client_location::{open_client, open_client_in};
pub use client_registry::{ClientDirectory, client_directories, client_directories_in};
pub use constants::CLIENT_STATE_DIRECTORY_ENV;
pub use constants::{DEFAULT_TAB_NAME, MAX_TAB_NAME_BYTES, MAX_TABS};
pub use constants::{FIRST_TAB_REVISION, TAB_FILE, TAB_SCHEMA_VERSION};
pub use document::{DocumentPane, LayoutCheckpoint, LocalTabView, TabDocument, TabViewMode};
pub use location::default_directory;
pub use store::TabStore;

#[cfg(test)]
#[path = "../unit_tests/lib.rs"]
mod unit_tests;

//! Trusted plugin manifest schema 1. No role, sandbox, grant or process profile.

mod command;
mod manifest;
mod validation;

pub use command::{Action, Command, WebMetadata};
pub use manifest::{MANIFEST_FILE, MAX_MANIFEST_BYTES, SCHEMA_VERSION, TrustedPluginManifest};
pub use validation::{ManifestError, load, parse};
pub use yolo_plugin_api::{ClientApiContract, Platform};

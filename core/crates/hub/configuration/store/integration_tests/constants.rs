#[path = "../test_support/constants.rs"]
mod shared;
pub use shared::*;

pub const HUB_DIRECTORY: &str = "hub";
pub const HOST_DIRECTORY: &str = "host";
pub const MOVED_DIRECTORY: &str = "moved-configuration";
pub const CLONE_DIRECTORY: &str = "configuration-copy";
pub const RECORD_FILE: &str = "configuration.json";
pub const LOCK_FILE: &str = "configuration.lock";
pub const NAMESPACE: &str = "configuration-test";
pub const INVALID_DOCUMENT: &[u8] = b"invalid-configuration";

#[path = "record.rs"]
mod record;
mod resume_identity;
#[path = "store.rs"]
mod store;
#[path = "store_repair.rs"]
mod store_repair;
#[path = "store_support.rs"]
mod store_support;

#[cfg(any(test, feature = "runtime-component-tests"))]
#[path = "../../test_support/authority/runtime_api.rs"]
mod runtime_test_api;

pub use record::{CodexAuthorityStatus, CodexAuthorityVersion, CodexBindingAuthority};
pub use store::CodexBindingAuthorityStore;

pub(crate) const AUTHORITY_FILE_NAME: &str = "codex-binding-authority.json";
pub(crate) const AUTHORITY_SCHEMA_VERSION: u16 = 4;

#[cfg(test)]
#[cfg(any(not(test), feature = "yolo-unit-tests"))]
#[path = "../../unit_tests/authority/tests.rs"]
mod tests;

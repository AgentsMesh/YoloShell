mod checkpoint;
mod constants;
mod document_wire;
mod intent;
mod intents;
mod open;
mod record;
mod recovery;
mod store;
mod write;

pub use constants::{JOURNAL_FILE, JOURNAL_SCHEMA_VERSION, MAX_PENDING_CREATIONS};
pub use intent::CreationIntent;
pub use record::PendingFinalization;
pub use store::CreationJournal;

#[cfg(all(test, feature = "yolo-integration-tests"))]
#[path = "../integration_tests/lib.rs"]
mod integration_tests;
#[cfg(test)]
#[path = "../test_support/checkpoints.rs"]
mod test_hooks;
#[cfg(all(test, not(feature = "yolo-integration-tests")))]
#[path = "../unit_tests/lib.rs"]
mod unit_tests;

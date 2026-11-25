#[path = "../test_support/constants.rs"]
mod shared;
pub use shared::*;

pub const TEST_DIRECTORY: &str = "projection";
pub const MOVED_DIRECTORY: &str = "moved-projection";
pub const CLONED_DIRECTORY: &str = "cloned-projection";
pub const STATE_FILE: &str = "agent-projection.json";
pub const RETAINED_LABEL: &str = "projection-test";
pub const AGENT_DIRECTORY: &str = "/remote/agent/项目";
pub const CHANGED_AGENT_DIRECTORY: &str = "/remote/agent/another project";
pub const ENTRIES_FIELD: &str = "entries";
pub const SNAPSHOT_FIELD: &str = "snapshot";
pub const SNAPSHOT_SCHEMA_FIELD: &str = "schema_version";

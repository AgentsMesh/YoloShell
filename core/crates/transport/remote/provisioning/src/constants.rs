pub const PROVISIONING_DIRECTORY: &str = "hub-provisioning";
pub(crate) const RECORD_FILE: &str = "record.json";
pub(crate) const LOCK_SUFFIX: &str = ".lock";
pub(crate) const LOCK_BYTES: u64 = 4096;
pub(crate) const SCHEMA: u16 = 1;
pub(crate) const ROOT_NAMESPACE: &str = "Hub provisioning operations";
pub(crate) const ENTRY_NAMESPACE: &str = "Hub provisioning operation";
pub(crate) const PARENT_NAMESPACE: &str = "Hub provisioning broker";
pub(crate) const INVALID: &str = "Hub provisioning record is inconsistent";
pub(crate) const CHANGED: &str = "Hub provisioning storage identity changed";
pub(crate) const CONFLICT: &str = "Hub provisioning request identity conflicts";
pub(crate) const BUSY: &str = "Hub provisioning operation is already owned";
pub(crate) const UNKNOWN: &str = "Hub provisioning outcome is unknown";
pub(crate) const INVALID_CHANNEL: &str = "Hub provisioning SSH channel identity changed";
pub(crate) const COMMAND_FAILED: &str =
    "Hub provisioning command did not produce a complete result";
pub(crate) const SUCCESS: u32 = 0;
pub(crate) const DEADLINE: &str = "Hub provisioning deadline expired";

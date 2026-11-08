pub const CONFIGURATION_DIRECTORY: &str = "configuration";
pub(crate) const RECORD_FILE: &str = "configuration.json";
pub(crate) const LOCK_FILE: &str = "configuration.lock";
pub(crate) const LOCK_BYTES: u64 = 4_096;
pub(crate) const NAMESPACE: &str = "Hub configuration";
pub(crate) const BUSY: &str = "Hub configuration is already owned";
pub(crate) const WRONG_IDENTITY: &str =
    "Hub configuration belongs to another identity or directory";
pub(crate) const SEED_CONFLICT: &str =
    "explicit peer configuration differs from persisted Hub configuration; change peers through the Hub management API or reset the persisted configuration explicitly";
pub(crate) const REVISION_CONFLICT: &str = "Hub configuration revision changed";
pub(crate) const REVISION_EXHAUSTED: &str = "Hub configuration revision exhausted";
pub(crate) const REPLACED_RECORD: &str = "Hub configuration file changed outside its owner";
pub(crate) const LOST: &str = "Hub configuration storage is no longer usable";
pub(crate) const PRIVATE_RECORD_SCHEMA: u16 = 2;
pub(crate) const INVALID_PRIVATE_SCHEMA: &str =
    "Hub configuration private record schema is invalid";

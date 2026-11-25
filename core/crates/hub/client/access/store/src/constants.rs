pub const CLIENT_ACCESS_DIRECTORY: &str = "client-access";
pub(crate) const RECORD_FILE: &str = "access.json";
pub(crate) const LOCK_FILE: &str = "access.lock";
pub(crate) const LOCK_BYTES: u64 = 4_096;
pub(crate) const PRIVATE_RECORD_SCHEMA: u16 = 2;
pub(crate) const REVISION_STEP: u64 = 1;
pub(crate) const NAMESPACE: &str = "Hub public client access";
pub(crate) const BUSY: &str = "Hub public client access is already owned";
pub(crate) const WRONG_IDENTITY: &str =
    "Hub public client access belongs to another entry or directory";
pub(crate) const SEED_CONFLICT: &str =
    "public client access seed conflicts with persisted authorization";
pub(crate) const REVISION_CONFLICT: &str = "public client access revision changed";
pub(crate) const REVISION_EXHAUSTED: &str = "public client access revision exhausted";
pub(crate) const REPLACED_RECORD: &str = "public client access file changed outside its owner";
pub(crate) const LOST: &str = "public client access storage is no longer usable";
pub(crate) const INVALID_SCHEMA: &str = "public client access private schema is invalid";

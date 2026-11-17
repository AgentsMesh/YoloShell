pub const REMOTE_BOOTSTRAP_SCHEMA_VERSION: u16 = 1;
pub const ENROLLMENT_RECOVERY_SCHEMA: u16 = 1;
pub const REMOTE_BOOTSTRAP_ARGUMENT: &str = "--remote-bootstrap";
pub const REMOTE_BOOTSTRAP_RECOVERY_ARGUMENT: &str = "--recover-remote-bootstrap";
pub const MAX_BOOTSTRAP_DOCUMENT_BYTES: usize = 16 * 1024;
pub(crate) const FIRST_CONFIGURATION_REVISION: u64 = 1;

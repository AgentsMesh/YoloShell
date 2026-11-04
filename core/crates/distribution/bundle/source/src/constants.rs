pub(crate) const MAX_MANIFEST_BYTES: u64 = 64 * 1024;
pub(crate) const WRITE_ACCESS: u32 = 0o022;
pub(crate) const EXECUTE_ACCESS: u32 = 0o111;
pub(crate) const REGULAR_LINKS: u64 = 1;
pub(crate) const ROOT_USER: u32 = 0;
pub(crate) const INVALID: &str = "bundle source metadata is unsafe";
pub(crate) const CHANGED: &str = "bundle source identity changed";
pub(crate) const MANIFEST_MISMATCH: &str = "bundle source manifest does not match its descriptor";

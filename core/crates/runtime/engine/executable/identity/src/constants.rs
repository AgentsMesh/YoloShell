pub(crate) const HASH_BUFFER_BYTES: usize = 64 * 1024;
pub(crate) const LENGTH_SENTINEL_BYTES: usize = 1;
pub(crate) const EXECUTABLE_MODE: u32 = 0o111;
pub(crate) const INVALID_IMAGE: &str =
    "running executable is not a bounded regular executable image";
pub(crate) const IMAGE_CHANGED: &str = "running executable changed during identity verification";
pub(crate) const WRONG_PROCESS: &str = "running executable evidence belongs to another process";
pub(crate) const INVALID_PATH: &str = "running executable path is not absolute";
pub(crate) const HASH_FAILED: &str = "read retained running executable image";

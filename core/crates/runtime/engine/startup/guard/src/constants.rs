pub const ARGUMENT_NAME: &str = "startup-guard";
pub const ARGUMENT: &str = "--startup-guard";
pub const COMMIT_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(30);
pub const CANCELLED: &str = "service startup was not committed";
pub(crate) const VERSION: u16 = 2;
pub(crate) const MAX_FRAME: usize = 2048;
pub const MAX_ENCODED_FRAME_BYTES: usize = MAX_FRAME + std::mem::size_of::<u32>();
pub(crate) const SENTINEL: usize = 1;
#[cfg(test)]
pub(crate) const INVALID_FRAME_BYTE: u8 = b'\n';
pub(crate) const INVALID_FRAME: &str = "invalid service startup frame";

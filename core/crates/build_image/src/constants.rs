pub const BUILD_IMAGE_SCHEMA_VERSION: u16 = 1;
pub const MAX_EXECUTABLE_IMAGE_BYTES: u64 = 512 * 1024 * 1024;
pub const SHA256_HEX_BYTES: usize = 64;
pub(crate) const INVALID_SCHEMA: &str = "unsupported executable image schema";
pub(crate) const INVALID_DIGEST: &str =
    "executable image SHA-256 must be canonical lowercase hexadecimal";
pub(crate) const INVALID_SIZE: &str = "executable image size is outside supported bounds";

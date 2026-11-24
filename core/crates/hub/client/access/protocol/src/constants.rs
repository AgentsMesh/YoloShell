pub const ACCESS_SCHEMA_VERSION: u16 = 1;
pub const FIRST_ACCESS_REVISION: u64 = 1;
pub const ACCESS_REVISION_STEP: u64 = 1;
pub const MAX_CLIENTS: usize = 128;
pub const MAX_CLIENT_ORIGINS: usize = 256;
pub const ED25519_PUBLIC_KEY_BYTES: usize = 32;

pub(crate) const EMPTY_PUBLIC_KEY: [u8; ED25519_PUBLIC_KEY_BYTES] = [0; ED25519_PUBLIC_KEY_BYTES];
pub(crate) const INVALID_EXPIRY: u64 = 0;
pub(crate) const IDENTITY_ERROR: &str = "invalid public client identity";
pub(crate) const GRANT_ERROR: &str = "invalid public client grant";
pub(crate) const CONFIGURATION_ERROR: &str = "invalid public client access configuration";
pub(crate) const DUPLICATE_CLIENT_ERROR: &str = "duplicate public client identity or key";
pub(crate) const DUPLICATE_ORIGIN_ERROR: &str = "duplicate public client origin grant";
pub(crate) const CAPACITY_ERROR: &str = "public client access capacity exceeded";
pub(crate) const EXPIRED_ERROR: &str = "public client grant has expired";

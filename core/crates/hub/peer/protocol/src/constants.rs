pub const PEER_PROTOCOL_VERSION: u16 = 7;
/// Peer catalogue payload shape. A different value cannot be verified against
/// the current canonical encoding and is republished on the next refresh.
pub const CATALOGUE_SCHEMA_VERSION: u16 = 2;
pub const MAX_DIRECTORY_ORIGINS: usize = 128;
pub const MAX_DIRECTORY_ROUTES: usize = 4096;
pub const MAX_CATALOGUE_RESOURCES: usize = 8_192;
pub const MAX_CATALOGUE_LIFETIME_MS: u64 = 60_000;
pub const MAX_CALL_LIFETIME_MS: u64 = 30_000;
pub const MAX_TERMINAL_LIFETIME_MS: u64 = 30_000;
pub const MAX_CLOCK_SKEW_MS: u64 = 5_000;
pub const FIRST_REVISION: u64 = 1;
pub const FIRST_STREAM_SEQUENCE: u64 = 1;
pub const PEER_METHOD_TAG: &str = "method";
pub const PEER_RESULT_TAG: &str = "result";
pub(crate) const SIGNATURE_DOMAIN: &str = "yolo.hub.peer.capnp.document.v1";

use std::time::Duration;

pub const START_HUB_COMMAND: &str = "start-hub";
pub const AUTHORITY_STDIO_ARGUMENT: &str = "--authority-stdio";
pub const AUTHORITY_STDIO_NAME: &str = "authority-stdio";
pub(crate) const VERSION: u16 = 2;
pub(crate) const DOMAIN: &[u8] = b"Yolo broker launch authority Cap'n Proto v2\0";
pub(crate) const MAX_FRAME_BYTES: usize = 1_048_576;
pub(crate) const MAX_CONFIG_BYTES: u64 = 2_097_152;
pub(crate) const HANDSHAKE_TIMEOUT: Duration = Duration::from_secs(5);
pub(crate) const INVALID: &str = "invalid Hub broker launch authority frame";
pub(crate) const REVOKED: &str = "Hub broker launch authority is revoked or expired";
pub(crate) const CHILD_MISMATCH: &str =
    "Hub broker launch permit is not bound to this captured child";
pub(crate) const SCOPE_MISMATCH: &str = "Hub broker launch scope differs from authorized state";
pub(crate) const CLOSED: &str = "Hub broker launch authority session has completed";
pub(crate) const POISONED: &str = "Hub broker launch authority state was poisoned";
pub(crate) const INVALID_CREDENTIALS: &str = "invalid private broker credential envelope";
pub(crate) const COUNTER_STEP: u64 = 1;

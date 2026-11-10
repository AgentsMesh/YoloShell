use std::time::Duration;
use yolo_hub_client_access_protocol::ED25519_PUBLIC_KEY_BYTES;

pub const NOW_MS: u64 = 1_000;
pub const EXPIRES_AT_MS: u64 = 2_000;
pub const BEFORE_EXPIRY_MS: u64 = EXPIRES_AT_MS - CLOCK_TICK_MS;
pub const CLOCK_TICK_MS: u64 = 1;
pub const REVISION_STEP: u64 = 1;
pub const REVISION_GAP: u64 = 2;
pub const FIRST_CLIENT: usize = 0;
pub const CLIENT_KEY: [u8; ED25519_PUBLIC_KEY_BYTES] = [1; ED25519_PUBLIC_KEY_BYTES];
pub const OTHER_KEY: [u8; ED25519_PUBLIC_KEY_BYTES] = [2; ED25519_PUBLIC_KEY_BYTES];
pub const WAKE_TIMEOUT: Duration = Duration::from_secs(2);
pub const POISON_MESSAGE: &str = "injected access registry owner failure";

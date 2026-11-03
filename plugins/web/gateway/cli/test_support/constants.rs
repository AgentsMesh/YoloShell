use std::time::Duration;

pub const BINARY_ENV: &str = "YOLO_WEB_CLI_TEST_BINARY";
pub const TIMEOUT: Duration = Duration::from_secs(10);
pub const POLL: Duration = Duration::from_millis(5);
pub const GRANT_LIFETIME_MS: u64 = 60_000;
pub const MAX_LOG_BYTES: u64 = 64 * 1_024;
pub const PROFILE_OPTION: &str = "--profile";
pub const KEY_OPTION: &str = "--client-key";
pub const TOKEN_OPTION: &str = "--token-file";
pub const ORIGIN_OPTION: &str = "--origin";
pub const LISTEN_OPTION: &str = "--listen";
pub const CAPACITY_OPTION: &str = "--max-connections";
pub const LIFETIME_OPTION: &str = "--session-seconds";
pub const ORIGIN: &str = "http://127.0.0.1:3000";
pub const TOKEN: &str = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";
pub const READY_PREFIX: &str = "Yolo Web gateway listening on ws://";
pub const SOCKET_FIELD: &str = "socket";
pub const HUB_FIELD: &str = "hub_id";
pub const HUB_KEY_FIELD: &str = "hub_public_key";
pub const CLIENT_FIELD: &str = "client";
pub const HELLO_FIELD: &str = "hello";
pub const METHODS_FIELD: &str = "methods";
pub const PRIVATE_MODE: u32 = 0o600;
pub const DIRECTORY_MODE: u32 = 0o700;
pub const PUBLIC_MODE: u32 = 0o644;
pub const WRONG_KEY_BYTES: [u8; 32] = [1; 32];

pub fn deadline() -> tokio::time::Instant {
    tokio::time::Instant::now() + TIMEOUT
}

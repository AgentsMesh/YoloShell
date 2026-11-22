pub const HUB_DIRECTORY: &str = "hub";
pub const HOST_DIRECTORY: &str = "host";
pub const BROKER_DIRECTORY: &str = "broker";
pub const UNUSED_HOST_SOCKET: &str = "absent-host.sock";
pub const SAVED_RECORD: &str = "saved-setup-record";
pub const RECORD_FILE: &str = "record.json";
pub const SOCKET_MODE: u32 = 0o600;
pub const TIMEOUT: std::time::Duration = std::time::Duration::from_secs(10);
pub const POLL_DELAY: std::time::Duration = std::time::Duration::from_millis(5);
pub const ONE: usize = 1;
pub const GENERATION: u64 = 1;
pub const INVALID_SOURCE: &str = "/does-not-exist/launch-source.json";
pub const SHA: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
pub fn deadline() -> tokio::time::Instant {
    tokio::time::Instant::now() + TIMEOUT
}

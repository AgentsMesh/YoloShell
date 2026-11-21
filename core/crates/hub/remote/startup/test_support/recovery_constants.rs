pub const STATE_DIRECTORY: &str = "hub";
pub const HOST_DIRECTORY: &str = "host";
pub const ABSENT_INSTALL: &str = "must-not-install-or-start";
pub const MOVED_ENTRY: &str = "saved-entry";
pub const RECORD_FILE: &str = "record.json";
pub const LISTENER_BUSY: &str = "enrollment fixture socket is occupied";
pub const TIMEOUT: std::time::Duration = std::time::Duration::from_secs(5);
pub const PORT: u16 = 24321;
pub const EXPECTED_CONFIGURE_COUNT: usize = 1;
pub const EXPECTED_CONNECTION_COUNT: usize = 1;

pub fn deadline() -> tokio::time::Instant {
    tokio::time::Instant::now() + TIMEOUT
}

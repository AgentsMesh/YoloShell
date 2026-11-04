pub const BROKER: &str = "broker";
pub const SOURCE: &str = "source";
pub const PROFILE: &str = "remote";
pub const STATE: &str = "/remote/hub";
pub const HOME: &str = "/remote/home";
pub const DIGEST: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
pub const PRIVATE_MODE: u32 = 0o700;
pub const FILE_MODE: u32 = 0o600;
pub const RECORD_FILE: &str = "record.json";
pub const PHASE_FIELD: &str = "phase";
pub const PUBLISHED_PHASE: &str = "published";
pub const PUBLISHING_PHASE: &str = "publishing";
pub const LOST: &str = "injected provisioning acknowledgement loss";
pub const REVOKED: &str = "injected profile revocation";
pub const SAVED: &str = "saved";
pub const TIMEOUT: std::time::Duration = std::time::Duration::from_secs(10);
pub const SUCCESS: u32 = 0;
pub const ONE: usize = 1;
pub fn deadline() -> tokio::time::Instant {
    tokio::time::Instant::now() + TIMEOUT
}

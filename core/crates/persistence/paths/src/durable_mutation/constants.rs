use std::time::Duration;

pub const SESSION_LIFECYCLE_BUSY: &str = "durable reboot session lifecycle lock is busy";
pub const SESSION_DIRECTORY_MISMATCH: &str = "durable reboot session directory identity changed";
pub const SESSION_ID_INVALID: &str = "durable reboot session identity is invalid";
pub const PENDING_INTENT: &str = "durable reboot migration is pending";
pub const PARTICIPANT_PATH_INVALID: &str = "durable reboot participant path is invalid";
const SESSION_DIRECTORY_LOCK_TIMEOUT_SECONDS: u64 = 8;
const SESSION_DIRECTORY_LOCK_RETRY_MILLISECONDS: u64 = 1;
pub const SESSION_DIRECTORY_LOCK_TIMEOUT: Duration =
    Duration::from_secs(SESSION_DIRECTORY_LOCK_TIMEOUT_SECONDS);
pub const SESSION_DIRECTORY_LOCK_RETRY: Duration =
    Duration::from_millis(SESSION_DIRECTORY_LOCK_RETRY_MILLISECONDS);
pub const SESSION_PARTICIPANT_LIFECYCLE_LOCK_TIMEOUT: Duration = SESSION_DIRECTORY_LOCK_TIMEOUT;
pub const SESSION_PARTICIPANT_LIFECYCLE_LOCK_RETRY: Duration = SESSION_DIRECTORY_LOCK_RETRY;

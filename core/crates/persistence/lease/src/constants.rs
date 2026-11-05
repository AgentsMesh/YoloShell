pub const DEFAULT_TTL_MS: u64 = 10_000;
pub const MIN_TTL_MS: u64 = 1_000;
pub const MAX_TTL_MS: u64 = 30_000;
pub const RENEW_AFTER_DIVISOR: u64 = 3;
pub(super) const AUTHORIZATION_FAILED_MESSAGE: &str = "writer lease authorization failed";
#[cfg(any(test, feature = "yolo-test-support"))]
pub const EXPIRED_DEADLINE_OFFSET: std::time::Duration = std::time::Duration::from_millis(1);
pub mod code {
    pub const HELD_BY_OTHER: &str = "held_by_other";
    pub const LEASE_EXPIRED: &str = "lease_expired";
    pub const NOT_HOLDER: &str = "not_holder";
    pub const STALE_GENERATION: &str = "stale_generation";
    pub const CONTROLLER_INSTANCE_MISMATCH: &str = "controller_instance_mismatch";
    pub const SESSION_NOT_RUNNING: &str = "session_not_running";
    pub const GENERATION_OVERFLOW: &str = "generation_overflow";
}

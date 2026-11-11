use std::time::Duration;

pub const HOST_SERVE_COMMAND: &str = "serve";
pub const HOME_ARGUMENT: &str = "--home";
pub const SOCKET_ARGUMENT: &str = "--socket";
pub const HOST_READY_TIMEOUT: Duration = Duration::from_secs(8);
pub const HOST_READY_POLL_INTERVAL: Duration = Duration::from_millis(25);
pub const BROKER_RECONNECT_INITIAL_INTERVAL: Duration = Duration::from_millis(25);
pub const BROKER_RECONNECT_MAX_INTERVAL: Duration = Duration::from_millis(500);
pub const READ_ONLY_POLL_INTERVAL: Duration = Duration::from_millis(500);
pub const CONTROL_RECONNECT_DELAY: Duration = Duration::from_millis(25);
pub const CONTROL_CONNECTION_INVARIANT: &str = "control connection initialized";
pub const LEASE_CLEANUP_TIMEOUT: Duration = Duration::from_secs(1);
pub const LEASE_CLEANUP_POLL_INTERVAL: Duration = Duration::from_millis(25);
pub const LEASE_CLEANUP_POLL_ATTEMPTS: usize = 4;
pub const TAKEOVER_MAX_ATTEMPTS: usize = 4;
pub const INPUT_BUFFER_BYTES: usize = 8 * 1024;
pub const PENDING_INPUT_BUFFER_COUNT: usize = 8;
pub const PENDING_INPUT_BYTES: usize = INPUT_BUFFER_BYTES * PENDING_INPUT_BUFFER_COUNT;
pub const TERMINAL_RESET: &[u8] = b"\x1bc";
pub const HOST_ALREADY_RUNNING_CODE: &str = "conflict";

pub mod error_code {
    pub const HELD_BY_OTHER: &str = "held_by_other";
    pub const LEASE_EXPIRED: &str = "lease_expired";
    pub const NOT_HOLDER: &str = "not_holder";
    pub const STALE_GENERATION: &str = "stale_generation";
    pub const CONTROLLER_INSTANCE_MISMATCH: &str = "controller_instance_mismatch";
}

#[cfg(test)]
pub mod test_value {
    pub const NON_MATCHING_CODE_PREFIX: &str = "not_the_";
    pub const SIMILAR_CODE_SUFFIX: &str = "_now";
}

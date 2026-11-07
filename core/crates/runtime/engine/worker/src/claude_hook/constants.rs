use std::time::Duration;

pub(super) const SOCKET_LIVE_ERROR: &str = "Claude hook socket is already live";
pub(super) const SOCKET_BIND_CONTEXT: &str = "bind retained Claude hook socket";
pub(super) const SOCKET_LISTENER_UNAVAILABLE: &str = "Claude hook listener is unavailable";
pub(super) const SOCKET_PARENT_MISSING: &str = "Claude hook socket has no parent";
pub(super) const SOCKET_PROBE_TIMEOUT: Duration = Duration::from_millis(250);
pub(super) const REQUEST_TIMEOUT: Duration = Duration::from_secs(5);
pub(super) const RESPONSE_TIMEOUT: Duration = Duration::from_secs(5);
pub(super) const REQUEST_TIMEOUT_CONTEXT: &str = "Claude hook request timed out";
pub(super) const RESPONSE_TIMEOUT_CONTEXT: &str = "Claude hook response timed out";
pub(super) const PEER_UID_ERROR: &str = "Claude hook peer user does not match this Worker";
pub(super) const PEER_PID_ERROR: &str = "Claude hook peer process identity is unavailable";
pub(super) const MAX_HOOK_CLIENTS: usize = 16;
pub(super) const HOOK_ADMISSION_TASK: &str = "claude_hook_admission";
pub(super) const HOOK_CONNECTION_TASK: &str = "claude_hook_connection";
pub(super) const HOOK_READINESS_RETENTION_TASK: &str = "claude_hook_readiness_retention";
pub(super) const ACCEPT_FAILURE_INCREMENT: u32 = 1;

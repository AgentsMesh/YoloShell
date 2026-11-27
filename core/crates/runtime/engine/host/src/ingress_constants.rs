use std::time::Duration;

pub(super) const HOST_HOME_LOCK_LABEL: &str = "home";
pub(super) const HOST_SOCKET_LOCK_LABEL: &str = "socket endpoint";
pub(super) const HOST_HOME_NAMESPACE_LABEL: &str = "host home";
pub(super) const HOST_SESSIONS_NAMESPACE_LABEL: &str = "host sessions";
pub(super) const HOST_SOCKET_NAMESPACE_LABEL: &str = "host socket parent";
pub(super) const HOST_WORKER_SOCKET_NAMESPACE_LABEL: &str = "host worker sockets";
pub(super) const HOST_INSTANCE_LOCK_MAX_BYTES: u64 = 128;
pub(super) const HOST_INSTANCE_LOCK_PID_PREFIX: &str = "pid=";
pub(super) const HOST_INSTANCE_LOCK_COUNT: usize = 2;
pub(super) const MAX_HOST_ATTACHMENTS: usize = 128;
pub(super) const HOST_CONTROL_CLIENT_RESERVE: usize = 16;
pub(super) const MAX_HOST_CLIENTS: usize = MAX_HOST_ATTACHMENTS + HOST_CONTROL_CLIENT_RESERVE;
pub(super) const HOST_REQUEST_TIMEOUT: Duration = Duration::from_secs(5);
pub(super) const HOST_IDLE_SHUTDOWN_CHECK_TIMEOUT: Duration = Duration::from_secs(5);
pub(super) const HOST_IDLE_SHUTDOWN_QUIET_PERIOD: Duration = Duration::from_secs(2);
pub(super) const HOST_RESPONSE_DRAIN_TIMEOUT: Duration =
    crate::session_timing::SESSION_RESUME_CLIENT_TIMEOUT;
pub(super) const HOST_ACCEPT_INITIAL_BACKOFF: Duration = Duration::from_millis(100);
pub(super) const HOST_ACCEPT_MAX_BACKOFF: Duration = Duration::from_secs(2);
pub(super) const HOST_REQUEST_TIMEOUT_CONTEXT: &str = "host request timed out";
pub(super) const HOST_INGRESS_LOOP_PANIC_CONTEXT: &str = "host ingress event loop panicked";
pub(super) const HOST_RESPONSE_DRAIN_TIMEOUT_LOG: &str =
    "host response publication drain exceeded its shutdown deadline";
pub(super) const HOST_ATTACHMENT_LIMIT_MESSAGE: &str = "host attachment limit reached";

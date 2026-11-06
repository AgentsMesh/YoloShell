use std::time::Duration;

pub(crate) const DEADLINE_EXPIRED: &str = "Hub peer dial deadline expired";
pub(crate) const SSH_PROGRAM: &str = "ssh";
pub const PEER_STDIO_COMMAND: &str = "internal hub";
pub const PEER_STDIO_FLAG: &str = "--peer-stdio";
pub const PEER_SOCKET_FLAG: &str = "--peer-socket";
pub(crate) const REMOTE_YOLO: &str = "yolo";
pub(crate) const REMOTE_YOLO_FALLBACK: &str = "$HOME/.local/bin/yolo";
pub(crate) const SSH_SPAWN_FAILED: &str = "start the SSH peer transport";
pub(crate) const SSH_TARGET_MISSING: &str = "peer address is not an SSH destination";
pub(crate) const SSH_STDIO_UNAVAILABLE: &str = "SSH peer transport has no stdio pipes";
pub(crate) const SSH_FAILED: &str = "SSH peer transport failed";
pub(crate) const MAX_STDERR_BYTES: usize = 4_096;
pub(crate) const SHUTDOWN_TIMEOUT: Duration = Duration::from_secs(5);
pub(crate) const SSH_CONNECT_TIMEOUT_SECONDS: u64 = 10;
pub(crate) const SSH_ALIVE_INTERVAL_SECONDS: u64 = 15;
pub(crate) const SSH_ALIVE_COUNT_MAX: u64 = 3;

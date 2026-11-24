use std::time::Duration;
#[cfg(test)]
#[path = "test_constants.rs"]
mod test;
#[cfg(test)]
pub(super) use test::*;
pub const WORKER_AUTH_METHOD: &str = "worker_auth";
pub const WORKER_STATUS_METHOD: &str = "worker_status";
pub const WORKER_SHUTDOWN_METHOD: &str = "worker_shutdown";
pub(super) const AGENT_MONITOR_TASK: &str = "agent_monitor";
pub(super) const CODEX_AUTHORITY_PROJECTION_TASK: &str = "codex_authority_projection";
pub(super) const PTY_CUSTODY_TASK: &str = "pty_custody";
pub(super) const CONNECTION_TASK: &str = "connection";
#[cfg(test)]
pub(super) const INITIAL_INPUT_TASK: &str = "initial_input";
pub(super) const ACCEPT_TASK: &str = "accept";
pub(super) const PTY_READER_TASK: &str = "pty_reader";
pub(super) const CHILD_WAITER_TASK: &str = "child_waiter";
pub(super) const TASK_SUPERVISOR: &str = "task_supervisor";
pub(super) const PTY_READER_THREAD_PREFIX: &str = "yolo-pty-read";
pub(super) const PTY_OWNER_THREAD_PREFIX: &str = "yolo-pty-owner";
pub(super) const TASK_PANIC_DETAIL: &str = "task panicked";
pub(super) const TASK_STOPPED_UNEXPECTEDLY: &str = "task stopped before its owner";
pub(super) const PTY_READER_UNEXPECTED_EOF_DETAIL: &str =
    "PTY reader reached EOF before child exit was confirmed";
pub(super) const PTY_READER_RECOVERY_EXHAUSTED_DETAIL: &str =
    "PTY reader recovery exhausted; waiting for confirmed child exit";
pub(super) const PTY_READER_FAILURE_RETRY_DETAIL: &str =
    "PTY reader recovery remains pending after repeated failures";
pub(super) const PTY_READER_NAMESPACE_LOST_DETAIL: &str =
    "PTY reader detected worker namespace loss; draining continues to kernel EOF";
pub(super) const PTY_PENDING_OUTPUT_CONFLICT_DETAIL: &str =
    "PTY reader attempted to replace uncommitted output";
pub(super) const TERMINAL_INPUT_EMPTY_DETAIL: &str = "terminal input cannot be empty";
pub(super) const TERMINAL_INPUT_ID_REUSE_DETAIL: &str =
    "terminal input identifier was reused with different data";
pub(super) const TERMINAL_WRITER_CLOSED_DETAIL: &str = "terminal writer is closed";
pub(super) const TERMINAL_DIMENSIONS_INVALID_DETAIL: &str = "terminal dimensions must be non-zero";
pub(super) const TERMINAL_RUNTIME_PERSISTENCE_DETAIL: &str =
    "persist terminal dimensions in the exact runtime";
pub(super) const TERMINAL_PARSER_PANIC_DETAIL: &str =
    "terminal parser rejected durable output and was reset";
pub(super) const SCREEN_RESET_SKIPPED_CHUNKS: u64 = 1;
pub(super) const TERMINAL_PTY_CLOSED_DETAIL: &str = "terminal PTY is closed";
pub(super) const PTY_INPUT_INCOMPLETE_DETAIL: &str =
    "terminal input writer did not accept the complete request";
pub(super) const PTY_FD_FLAGS_READ_CONTEXT: &str = "read PTY file status flags";
pub(super) const PTY_FD_NONBLOCK_CONTEXT: &str = "enable bounded PTY input writes";
pub(super) const PTY_FD_UNAVAILABLE_DETAIL: &str =
    "PTY master does not expose a Unix file descriptor";
pub(super) const PTY_WRITER_DUPLICATE_CONTEXT: &str = "duplicate bounded PTY input descriptor";
pub(super) const PTY_READER_POLL_CONTEXT: &str = "wait for PTY output readiness";
pub(super) const PTY_READER_POLL_INVALID_DETAIL: &str =
    "PTY output descriptor became invalid while waiting for readiness";
pub(super) const PTY_READER_POLL_UNEXPECTED_DETAIL: &str =
    "PTY output readiness wait returned an unexpected result";
pub(super) const PTY_OUTPUT_RETRY_DETAIL: &str =
    "PTY output remains pending while durable journal commit is retried";
#[cfg(test)]
pub(super) const CHILD_IDENTITY_LOST_DETAIL: &str =
    "child identity disappeared after waiter failure";
pub const WORKER_MAX_REPLAY_BYTES: usize = 8 * 1024 * 1024;
pub(super) const PTY_READER_DRAIN_TIMEOUT: Duration = Duration::from_secs(2);
pub(super) const ORPHAN_CHECK_INTERVAL: Duration = Duration::from_millis(250);
pub(super) const ORPHAN_REASON: &str = "session persistence root was removed";
pub(super) const PTY_READ_BUFFER_BYTES: usize = 16 * 1024;
pub(super) const PTY_OUTPUT_BATCH_CHUNK_LIMIT: usize = 32;
pub(super) const PTY_SNAPSHOT_INTERVAL_CHUNKS: u64 = 32;
pub(super) const PTY_READER_EXIT_CONFIRM_TIMEOUT: Duration = Duration::from_millis(500);
pub(super) const PTY_READER_EXIT_POLL_INTERVAL: Duration = Duration::from_millis(10);
pub(super) const PTY_READER_UNCONFIRMED_EOF_REOPEN_LIMIT: u32 = 3;
pub(super) const PTY_READER_FAILURE_RESTART_LIMIT: u32 = 3;
pub(super) const NATURAL_EXIT_SHUTDOWN_DELAY: Duration = Duration::from_millis(100);
pub(super) const NATURAL_EXIT_ATTACHMENT_GRACE: Duration = Duration::from_secs(2);
pub(super) const NATURAL_EXIT_ATTACHMENT_POLL: Duration = Duration::from_millis(50);
pub(super) const NATURAL_EXIT_ATTACHMENT_DRAIN_MAX: Duration = Duration::from_secs(30);
pub(super) const NATURAL_EXIT_SHUTDOWN_THREAD: &str = "yolo-natural-exit";
pub(super) const EXPLICIT_EXIT_SHUTDOWN_DELAY: Duration = Duration::from_millis(50);
pub(super) const PROCESS_SCOPE_REFRESH_INTERVAL: Duration = Duration::from_millis(200);
pub(super) const STARTUP_CHILD_EXIT_TIMEOUT: Duration = Duration::from_secs(2);
pub(super) const STARTUP_CHILD_EXIT_POLL_INTERVAL: Duration = Duration::from_millis(10);
pub(super) const STARTUP_SCOPE_MONITOR_THREAD_NAME: &str = "yolo-startup-scope";
pub(super) const WORKER_EVENT_CHANNEL_CAPACITY: usize = 1024;
pub(super) const TERMINAL_SCROLLBACK_ROWS: usize = 10_000;
pub(super) const DELIVERED_INPUT_CACHE_LIMIT: usize = 1024;
pub(super) const UNIX_SOCKET_PATH_MAX_BYTES: usize =
    crate::paths::constants::UNIX_SOCKET_PATH_MAX_BYTES;
pub(super) const JOURNAL_APPEND_RECOVERY_ATTEMPTS: usize = 2;
pub(super) const JOURNAL_APPEND_FAILURE_CONTEXT: &str = "persist PTY output after journal recovery";
pub(super) const JOURNAL_RECONCILE_FAILURE_CONTEXT: &str = "reconcile uncertain output journal";
pub(super) const JOURNAL_COMPACTION_FAILURE_CONTEXT: &str = "compact output journal";
pub(super) const SUPERVISION_RESTART_INITIAL_DELAY: Duration = Duration::from_millis(100);
pub(super) const SUPERVISION_RESTART_MAX_DELAY: Duration = Duration::from_secs(2);
pub(super) const SUPERVISION_RESTART_MULTIPLIER: u32 = 2;
pub(super) const WORKER_OPERATION_TIMEOUT: Duration = Duration::from_secs(2);
pub(super) const WORKER_OPERATION_TIMEOUT_CONTEXT: &str = "worker socket operation timed out";
pub(super) const MAX_WORKER_ATTACHMENTS: usize = 64;
pub(super) const WORKER_CONTROL_CLIENT_RESERVE: usize = 8;
pub(super) const MAX_WORKER_CLIENTS: usize = MAX_WORKER_ATTACHMENTS + WORKER_CONTROL_CLIENT_RESERVE;
pub(super) const WORKER_ATTACHMENT_LIMIT_MESSAGE: &str = "worker attachment limit reached";
pub(super) const WORKER_AUTH_TIMEOUT_CONTEXT: &str = "worker authentication timed out";
pub(super) const WORKER_AUTH_FAILURE_CONTEXT: &str = "worker authentication failed";
pub(super) const WORKER_INSTANCE_LOCK_MAX_BYTES: u64 = 0;
pub(super) const WORKER_SOCKET_PROBE_TIMEOUT: Duration = Duration::from_millis(250);
#[cfg(test)]
pub(super) const WORKER_SOCKET_PERMISSION_MODE: u32 = crate::paths::constants::PRIVATE_FILE_MODE;
#[cfg(test)]
pub(super) const WORKER_SOCKET_PERMISSION_MASK: u32 = 0o7777;
pub(super) const WORKER_INSTANCE_BUSY_CONTEXT: &str = "another worker owns this session";
pub(super) const WORKER_SOCKET_LIVE_CONTEXT: &str =
    "worker socket is already accepting connections";
pub(super) const WORKER_SOCKET_NAMESPACE_MISMATCH_CONTEXT: &str =
    "worker launch socket does not belong to its retained Yolo home";
#[cfg(test)]
pub(super) const WORKER_SOCKET_PROBE_TIMEOUT_CONTEXT: &str =
    "worker socket liveness probe timed out";
#[cfg(test)]
pub(super) const WORKER_SOCKET_INDETERMINATE_CONTEXT: &str =
    "worker socket liveness is indeterminate";
#[cfg(test)]
pub(super) const WORKER_SOCKET_CHANGED_CONTEXT: &str =
    "worker socket changed during stale endpoint probe";
#[cfg(test)]
pub(super) const WORKER_SOCKET_TYPE_ERROR: &str = "worker IPC path is not a Unix socket";
#[cfg(test)]
pub(super) const WORKER_SOCKET_OWNER_ERROR: &str = "worker socket owner is unsafe";
#[cfg(test)]
pub(super) const WORKER_SOCKET_PERMISSION_ERROR: &str = "worker socket permissions are unsafe";
#[cfg(test)]
pub(super) const WORKER_SOCKET_SECURE_CONTEXT: &str = "secure bound worker socket";
#[cfg(test)]
pub(super) const WORKER_SOCKET_REMOVE_CONTEXT: &str = "remove stale worker socket";
pub(super) const WORKER_LAUNCH_RETIRE_CONTEXT: &str = "retire single-use worker launch file";
pub(super) const WORKER_LAUNCH_IDENTITY_ERROR: &str = "worker launch session identity mismatch";
pub(super) const WORKER_LAUNCH_SCHEMA_ERROR: &str = "unsupported worker launch schema";
pub(super) const WORKER_IDENTITY_CONTEXT: &str = "cannot establish worker identity";
pub(super) const BOOTSTRAP_INPUT_EMPTY_DETAIL: &str = "bootstrap input cannot be empty";
pub(super) const BOOTSTRAP_WRITE_ZERO_DETAIL: &str = "bootstrap input writer made no progress";
pub(super) const BOOTSTRAP_FAILURE_CONTEXT: &str = "bootstrap input delivery failed";
pub(super) const BOOTSTRAP_GATE_SPEC_ENV: &str = "YOLO_BOOTSTRAP_GATE_SPEC";
pub(super) const BOOTSTRAP_GATE_TIMEOUT: Duration = Duration::from_secs(5);
pub(super) const BOOTSTRAP_GATE_TIMEOUT_CONTEXT: &str = "bootstrap child gate timed out";
pub(super) const BOOTSTRAP_GATE_PEER_CONTEXT: &str = "bootstrap child peer identity mismatch";
pub(super) const BOOTSTRAP_GATE_HELLO_CONTEXT: &str = "bootstrap child hello mismatch";
pub(super) const BOOTSTRAP_GATE_CAPABILITY_CONTEXT: &str =
    "bootstrap child process capability mismatch";
pub(super) const BOOTSTRAP_GATE_REPLY_CONTEXT: &str = "invalid bootstrap gate reply";
#[cfg(test)]
pub(super) const BOOTSTRAP_TEST_BAD_ACK_ENV: &str = "YOLO_BOOTSTRAP_TEST_BAD_ACK";
pub(super) const BOOTSTRAP_SPEC_MISSING_CONTEXT: &str = "bootstrap child specification is missing";
pub(super) const BOOTSTRAP_SPEC_DECODE_CONTEXT: &str = "decode bootstrap child specification";
pub(super) const BOOTSTRAP_EXEC_FAILURE_CONTEXT: &str = "execute gated PTY command";
pub(super) const BOOTSTRAP_EXEC_REPORT_CONTEXT: &str = "report gated PTY exec failure";
pub(super) const BOOTSTRAP_EXEC_OBSERVER_REGISTER_CONTEXT: &str =
    "register gated PTY exec observer";
pub(super) const BOOTSTRAP_EXEC_OBSERVER_IDENTITY_CONTEXT: &str =
    "gated PTY identity changed during exec observer registration";
pub(super) const BOOTSTRAP_EXEC_OBSERVER_WAIT_CONTEXT: &str = "wait for gated PTY exec event";
pub(super) const BOOTSTRAP_EXEC_OBSERVER_TIMEOUT_CONTEXT: &str =
    "gated PTY exec observation timed out";
pub(super) const BOOTSTRAP_EXEC_OBSERVER_EVENT_CONTEXT: &str =
    "invalid gated PTY exec observer event";
pub(super) const BOOTSTRAP_EXEC_OBSERVER_JOIN_CONTEXT: &str = "join gated PTY exec observer";
pub(super) const BOOTSTRAP_EXEC_NOT_OBSERVED_CONTEXT: &str = "gated PTY command exited before exec";
pub(super) const BOOTSTRAP_EXEC_THREAD_CONTEXT: &str =
    "bootstrap exec thread does not belong to the gated PTY process";
pub(super) const BOOTSTRAP_EXEC_THREAD_CHANGED_CONTEXT: &str =
    "bootstrap exec moved to a different thread after authentication";
#[cfg(target_os = "linux")]
pub(super) const BOOTSTRAP_EXEC_TRACE_THREAD_NAME: &str = "yolo-bootstrap-exec-trace";
#[cfg(target_os = "linux")]
pub(super) const BOOTSTRAP_EXEC_TRACE_SPAWN_CONTEXT: &str = "spawn bootstrap exec trace thread";
#[cfg(target_os = "linux")]
pub(super) const BOOTSTRAP_EXEC_TRACE_READY_CONTEXT: &str =
    "bootstrap exec trace registration timed out";
#[cfg(target_os = "linux")]
pub(super) const BOOTSTRAP_EXEC_TRACE_JOIN_CONTEXT: &str = "bootstrap exec trace thread panicked";
#[cfg(target_os = "linux")]
pub(super) const BOOTSTRAP_EXEC_TRACE_CANCEL_CONTEXT: &str =
    "bootstrap exec trace cancellation timed out";
#[cfg(target_os = "linux")]
pub(super) const BOOTSTRAP_EXEC_TRACE_CONTROL_CONTEXT: &str = "control bootstrap exec tracee";
#[cfg(target_os = "linux")]
pub(super) const BOOTSTRAP_EXEC_TRACE_STOP_LOG: &str = "failed to stop bootstrap exec trace thread";
#[cfg(target_os = "linux")]
pub(super) const BOOTSTRAP_EXEC_OBSERVER_POLL_INTERVAL: Duration = Duration::from_millis(10);
#[cfg(target_os = "linux")]
pub(super) const BOOTSTRAP_EXEC_OBSERVER_CANCEL_TIMEOUT: Duration = Duration::from_millis(500);
#[cfg(not(any(target_os = "linux", target_os = "macos")))]
pub(super) const BOOTSTRAP_EXEC_OBSERVER_UNSUPPORTED_CONTEXT: &str =
    "gated PTY exec observation requires Linux or macOS";
pub(super) const WORKER_NOT_READY_DETAIL: &str = "worker bootstrap is not ready";
pub(super) const CONTROLLER_PERSISTENCE_CONTEXT: &str = "persist controller fencing generation";
pub(super) const PTY_PROOF_CAPTURE_CONTEXT: &str = "capture PTY session proof";

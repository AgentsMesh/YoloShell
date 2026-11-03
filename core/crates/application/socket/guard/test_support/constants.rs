use std::time::Duration;

pub(super) const TEST_SOURCE_DIRECTORY_ENVIRONMENT: &str = "TEST_SRCDIR";
pub(super) const TEST_WORKSPACE_ENVIRONMENT: &str = "TEST_WORKSPACE";
pub(super) const TEST_EXECUTABLE_ENVIRONMENT: &str = "YOLO_TEST_EXECUTABLE";
pub(super) const TEST_FILTER_ENVIRONMENT: &str = "TESTBRIDGE_TEST_ONLY";
pub(super) const GUARD_MODE_ENVIRONMENT: &str = "YOLO_SOCKET_ROOT_GUARD_MODE";
pub(super) const LAUNCH_EXECUTABLE_ENVIRONMENT: &str = "YOLO_SOCKET_ROOT_GUARD_EXECUTABLE";
pub(super) const LAUNCH_SOCKET_ROOT_ENVIRONMENT: &str = "YOLO_SOCKET_ROOT_GUARD_SOCKET_ROOT";
pub(super) const WATCHDOG_MODE: &str = "watchdog";
pub(super) const LAUNCHER_MODE: &str = "launcher";
pub(super) const EXECUTABLE_MODE_MASK: u32 = 0o111;
pub(crate) const ERROR_PREFIX: &str = "isolated Rust test runner";
pub(super) const EXECUTABLE_ERROR: &str = "test executable is unavailable";
pub(super) const EXECUTION_CONTEXT: &str = "execute isolated Rust test";
pub(super) const WATCHDOG_CONTEXT: &str = "isolated test watchdog failed";
pub(super) const WATCHDOG_EARLY_EXIT_DETAIL: &str = "isolated test watchdog exited early";
pub(super) const WATCHDOG_PROTOCOL_DETAIL: &str = "isolated test watchdog protocol is invalid";
pub(super) const WATCHDOG_PIPE_DETAIL: &str = "isolated test watchdog control pipe is unavailable";
pub(super) const WATCHDOG_REAP_DETAIL: &str = "reap isolated test watchdog";
pub(super) const WATCHDOG_EXIT_DETAIL: &str = "isolated test watchdog exited unsuccessfully";
pub(super) const WATCHDOG_TREE_DETAIL: &str = "isolated test process tree did not settle";
pub(super) const WATCHDOG_REFRESH_DETAIL: &str = "isolated test process tracking failed";
pub(super) const WATCHDOG_HANDSHAKE_DETAIL: &str = "isolated test watchdog handshake timed out";
pub(super) const WATCHDOG_COMPLETION_DETAIL: &str = "isolated test watchdog cleanup timed out";
pub(super) const WATCHDOG_COMPLETION_WRITE_DETAIL: &str = "write isolated test watchdog completion";
pub(super) const WATCHDOG_DIAGNOSTIC_DETAIL: &str = "isolated test watchdog stderr";
pub(super) const WATCHDOG_DIAGNOSTIC_READ_DETAIL: &str = "read isolated test watchdog stderr";
pub(super) const WATCHDOG_DIAGNOSTIC_START_DETAIL: &str =
    "start isolated test watchdog stderr reader";
pub(super) const WATCHDOG_DIAGNOSTIC_THREAD_DETAIL: &str =
    "isolated test watchdog stderr reader panicked";
pub(super) const WATCHDOG_DIAGNOSTIC_CANCEL_DETAIL: &str =
    "isolated test watchdog stderr reader did not cancel";
pub(super) const WATCHDOG_DIAGNOSTIC_THREAD_NAME: &str = "yolo-watchdog-stderr";
pub(super) const WATCHDOG_OUTPUT_TRUNCATED_SUFFIX: &str = " [truncated]";
pub(super) const WATCHDOG_EXIT_STATUS_DETAIL: &str = "isolated test watchdog exit status";
pub(super) const WATCHDOG_EXIT_SIGNAL_DETAIL: &str = "isolated test watchdog exit signal";
pub(super) const LAUNCHER_CONTEXT: &str = "isolated test launcher failed";
pub(super) const LAUNCHER_PIPE_DETAIL: &str = "isolated test launcher pipe is unavailable";
pub(super) const LAUNCHER_START_DETAIL: &str = "isolated test launcher start capability is invalid";
pub(super) const LAUNCHER_READY_MARKER: &[u8; 1] = b"R";
pub(super) const LAUNCHER_READY_DETAIL: &str =
    "isolated test launcher user-space readiness is invalid";
pub(super) const LAUNCHER_ABORT_DETAIL: &str = "isolated test launcher did not abort";
pub(super) const CHILD_REAP_DETAIL: &str = "isolated test child did not settle";
pub(super) const TREE_SETTLEMENT_REPEATED_DETAIL: &str =
    "isolated test process tree settlement was already attempted";
pub(super) const RUNNER_SETTLEMENT_DETAIL: &str = "isolated test runner settlement failed";
pub(super) const CONTROL_READ_DETAIL: &str = "read isolated test watchdog control frame";
pub(super) const CONTROL_TIMEOUT_DETAIL: &str = "isolated test watchdog control frame timed out";
pub(super) const CONTROL_EOF_DETAIL: &str = "isolated test watchdog control pipe closed";
pub(super) const FAILURE_SEPARATOR: &str = "; ";
pub(super) const FRAME_ROOT: &str = "root";
pub(super) const FRAME_TRACK: &str = "track";
pub(super) const FRAME_TRACKED: &str = "tracked";
pub(super) const FRAME_START: &str = "start";
pub(super) const FRAME_FINISH: &str = "finish";
pub(super) const FRAME_COMPLETE: &str = "complete";
pub(super) const FRAME_FAILED: &str = "failed";
pub(super) const FRAME_SEPARATOR: char = '\t';
pub(super) const FRAME_TERMINATOR: char = '\n';
pub(super) const FRAME_TERMINATOR_BYTE: u8 = b'\n';
pub(super) const CAPABILITY_UUID_PARTS: usize = 2;
pub(super) const CAPABILITY_HEX_LENGTH: usize = 64;
pub(super) const PROGRAM_NAME_ARGUMENTS: usize = 1;
pub(super) const CONTROL_CHANNEL_CAPACITY: usize = 1;
pub(super) const MAXIMUM_CONTROL_FRAME_BYTES: u64 = 4_096;
pub(super) const CONTROL_FRAME_OVERFLOW_SENTINEL_BYTES: u64 = 1;
pub(super) const CONTROL_READ_BUFFER_BYTES: usize = 512;
pub(super) const MAXIMUM_WATCHDOG_DIAGNOSTIC_BYTES: usize = MAXIMUM_CONTROL_FRAME_BYTES as usize;
pub(super) const POLL_DESCRIPTOR_COUNT: libc::nfds_t = 1;
pub(super) const POLL_READY_COUNT: libc::c_int = 1;
pub(super) const INVALID_SYSTEM_RESULT: libc::c_int = -1;
pub(super) const MINIMUM_POLL_MILLISECONDS: u128 = 1;
pub(super) const PROCESS_POLL_INTERVAL: Duration = Duration::from_millis(10);
pub(super) const GRACEFUL_CLEANUP_WINDOW: Duration = Duration::from_millis(100);
pub(super) const WATCHDOG_SETTLEMENT_TIMEOUT: Duration = Duration::from_secs(8);
pub(super) const WATCHDOG_DISCOVERY_PREPARATION_TIMEOUT: Duration = WATCHDOG_SETTLEMENT_TIMEOUT;
pub(super) const WATCHDOG_FORCE_SETTLEMENT_TIMEOUT: Duration = WATCHDOG_SETTLEMENT_TIMEOUT;
pub(super) const WATCHDOG_HANDSHAKE_TIMEOUT: Duration = Duration::from_secs(5);
pub(super) const WATCHDOG_DROP_TIMEOUT: Duration = Duration::from_secs(10);
pub(super) const WATCHDOG_FORCE_REAP_TIMEOUT: Duration = Duration::from_secs(2);
pub(super) const WATCHDOG_SERVER_SETTLEMENT_BUDGET: Duration =
    WATCHDOG_DISCOVERY_PREPARATION_TIMEOUT
        .saturating_add(GRACEFUL_CLEANUP_WINDOW)
        .saturating_add(WATCHDOG_FORCE_SETTLEMENT_TIMEOUT);
pub(super) const WATCHDOG_COMPLETION_MARGIN: Duration = WATCHDOG_FORCE_REAP_TIMEOUT;
pub(super) const WATCHDOG_COMPLETION_TIMEOUT: Duration =
    WATCHDOG_SERVER_SETTLEMENT_BUDGET.saturating_add(WATCHDOG_COMPLETION_MARGIN);
pub(super) const WATCHDOG_DIAGNOSTIC_DRAIN_TIMEOUT: Duration = GRACEFUL_CLEANUP_WINDOW;
pub(super) const WATCHDOG_DIAGNOSTIC_CANCEL_TIMEOUT: Duration = WATCHDOG_FORCE_REAP_TIMEOUT;
pub(super) const LAUNCHER_ABORT_TIMEOUT: Duration = Duration::from_secs(2);
pub(super) const LAUNCHER_FORCE_REAP_TIMEOUT: Duration = Duration::from_secs(2);
pub(super) const CHILD_FORCE_REAP_TIMEOUT: Duration = Duration::from_secs(2);

#[cfg(test)]
#[cfg(any(not(test), feature = "yolo-unit-tests"))]
#[path = "../unit_tests/constants_tests.rs"]
mod tests;

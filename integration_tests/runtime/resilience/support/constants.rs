use std::time::Duration;

pub const CODEX_HOME_ENV: &str = "YOLO_CODEX_HOME";
pub const SHELL_ENV: &str = "SHELL";
pub const TEST_SHELL: &str = "/bin/sh";
pub const PROCESS_EXIT_TIMEOUT: Duration = Duration::from_secs(8);
pub const PROCESS_POLL_INTERVAL: Duration = Duration::from_millis(20);
pub const HOST_READY_TIMEOUT: Duration = Duration::from_secs(8);
pub const HOST_READY_POLL_INTERVAL: Duration = Duration::from_millis(25);
pub const HOST_STDERR_FILE_NAME: &str = "host.stderr.log";
pub const HOST_STDERR_MAX_TAIL_BYTES: usize = 16 * 1024;
pub const HOST_STDERR_MODE: u32 = 0o600;
pub const CLEANUP_RPC_TIMEOUT: Duration = yolo_session_timing::WORKER_AUTH_TIMEOUT;
pub const CLEANUP_CONNECT_TIMEOUT: Duration = Duration::from_secs(3);
pub const RUNTIME_GRACEFUL_EXIT_TIMEOUT: Duration =
    yolo_session_timing::WORKER_TERMINATION_PROTOCOL_TIMEOUT;
pub const CLEANUP_AUTH_REQUEST_ID: &str = "cleanup-auth";
pub const CLEANUP_TERMINATE_REQUEST_ID: &str = "cleanup-terminate";
pub const RUNFILE_ROOT_ENVIRONMENTS: &[&str] = &["RUNFILES_DIR", "TEST_SRCDIR"];
pub const RUNFILE_WORKSPACES: &[&str] = &["_main", "yolo"];
pub const HOST_BINARY_RUNFILE: &str = "core/crates/application/yolo/yolo";
pub const TEST_RUNTIME_ENVIRONMENT: &str = "YOLO_TEST_RUNTIME";
pub const TEST_RUNTIME_MISSING: &str = "Yolo test binary is unavailable in Bazel test inputs";
pub const TEST_RUNTIME_RESOLUTION_ERROR: &str = "resolve Yolo test binary";
pub const FAKE_CODEX_BINARY_RUNFILE: &str = "integration_tests/runtime/yolo-test-codex";
pub const FAKE_CLAUDE_BINARY_RUNFILE: &str = "integration_tests/runtime/yolo-test-claude";
pub const POWER_LOSS_ESCAPE_HELPER_FILTER: &str = "pre_capture_escaped_pty_holder_helper";
pub const TEST_HARNESS_NOCAPTURE_ARGUMENT: &str = "--nocapture";
pub const TEST_HARNESS_EXACT_ARGUMENT: &str = "--exact";

use std::time::Duration;

pub const BINARY_ENV: &str = "YOLO_TOOL_CLI_TEST_BINARY";
pub const TEST_WORKSPACE_ENV: &str = "TEST_WORKSPACE";
pub const HELP_ARGUMENT: &str = "--help";
pub const LONG_OPTION_PREFIX: &str = "--";
pub const USAGE_MARKER: &str = "Usage:";
pub const SUCCESS: i32 = 0;
pub const FAILURE: i32 = 1;
pub const USAGE_FAILURE: i32 = 2;
pub const SENTINEL: &[u8] = b"CLI fixture sentinel";
pub(crate) const CAPTURE_DIRECTORY: &str = "captures";
pub(crate) const MAX_OUTPUT_BYTES: u64 = 64 * 1024;
pub(crate) const OVERFLOW_SENTINEL: u64 = 1;
pub(crate) const EXIT_POLL: Duration = Duration::from_millis(10);
pub(crate) const DEADLINE: Duration = yolo_session_timing::RUST_INSPECTION_CLIENT_TIMEOUT;

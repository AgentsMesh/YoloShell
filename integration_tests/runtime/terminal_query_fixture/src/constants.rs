use std::time::Duration;

pub const BINARY_ENV: &str = "YOLO_TEST_TERMINAL_QUERY_BINARY";
pub const COMMAND_FILE: &str = "query-command.json";
pub const REPORT_FILE: &str = "query-report.json";
pub const READY_FILE: &str = "query-ready";
pub const READY_VALUE: &[u8] = b"ready";
pub const SEQUENCE_STEP: u64 = 1;
pub const MAX_COMMAND_BYTES: u64 = 16 * 1024;
pub const MAX_OUTPUT_BYTES: usize = 1024;
pub const MAX_RECEIVED_BYTES: usize = 8 * 1024;
pub const MAX_SETTLE_MS: u64 = 10_000;
pub const POLL: Duration = Duration::from_millis(10);
pub const LIFETIME: Duration = Duration::from_secs(120);
pub const ERROR: &str = "terminal query fixture contract violated";
pub(crate) const ARGUMENTS: usize = 2;
pub(crate) const DIRECTORY_ARGUMENT: usize = 1;
pub(crate) const READ_BYTES: usize = 1024;
pub(crate) const TEMP_REPORT_FILE: &str = "query-report-next.json";

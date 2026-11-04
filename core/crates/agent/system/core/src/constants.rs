use std::time::Duration;

pub use yolo_agent_adapter_contract::constants::{
    BINDING_SCHEMA_VERSION, ENVIRONMENT_ASSIGNMENT, ENVIRONMENT_NUL, HOME_ENV,
    MAX_PROVIDER_ENVIRONMENT_BYTES, MAX_PROVIDER_ENVIRONMENT_VARIABLES, MIN_PROCESS_PID,
    NODE_EXECUTABLES, RESERVED_ENVIRONMENT_PREFIX, YOLO_HUB_ID_ENV, YOLO_RUN_ID_ENV,
    YOLO_RUNTIME_EPOCH_ENV, YOLO_TERMINAL_ID_ENV, YOLO_WORKSPACE_ID_ENV,
};
pub use yolo_agent_contract::BINDING_FILE_NAME;
pub const DATA_HOME_FIELD: &str = "data_home";
pub const PROCESS_PID_FIELD: &str = "process_pid";
pub const PROCESS_START_TIME_FIELD: &str = "process_start_time";
pub const RECOVERY_AVAILABILITY_FIELD: &str = "recovery_availability";
pub const SCHEMA_VERSION_FIELD: &str = "schema_version";
pub const STATE_FIELD: &str = "state";
pub const JSONL_EXTENSION: &str = "jsonl";
pub const SHELL_ENV: &str = "SHELL";
pub const PATH_ENV: &str = "PATH";
pub const FALLBACK_SHELL: &str = "/bin/sh";
pub const LOGIN_SHELL_ARGUMENT: &str = "-l";
#[cfg(target_os = "macos")]
pub const ACCOUNT_RECORD_BUFFER_DEFAULT_BYTES: usize = 4 * 1_024;
#[cfg(target_os = "macos")]
pub const ACCOUNT_RECORD_BUFFER_GROWTH_FACTOR: usize = 2;
#[cfg(target_os = "macos")]
pub const ACCOUNT_RECORD_BUFFER_MAX_BYTES: usize = 1_024 * 1_024;
#[cfg(target_os = "macos")]
pub const ACCOUNT_RECORD_BUFFER_MIN_BYTES: usize = 1;
#[cfg(target_os = "macos")]
pub const ACCOUNT_LOOKUP_MAX_INTERRUPT_RETRIES: usize = 3;
pub const COMMAND_LINE_ENDING: char = '\n';
pub const COMMAND_PROGRAM_SLOT_COUNT: usize = 1;
pub const ENVIRONMENT_ASSIGNMENT_BYTE: u8 = b'=';
pub const NUL_BYTE: u8 = 0;
pub const MAX_PROCESS_INVENTORY_PIDS: usize = 32_768;
pub const MAX_FOREGROUND_PROCESS_IDS: usize = 512;
pub const MAX_PROCESS_ANCESTOR_DEPTH: usize = 64;
pub const MAX_PROCESS_ANCESTOR_STEPS: usize = 4_096;
pub const MAX_PROCESS_ARGV_BYTES: usize = 8 * 1_024 * 1_024;
pub const MAX_SINGLE_PROCESS_ARGV_BYTES: usize = 256 * 1_024;
pub const PROCESS_ARGV_READ_SENTINEL_BYTES: usize = 1;
pub const MAX_PROCESS_ENVIRONMENT_FIELDS: usize = 4_096;
pub const MAX_PROCESS_ENVIRONMENT_KEYS: usize = 64;
pub const MAX_PROCESS_ENVIRONMENT_VALUE_BYTES: usize = 4_096;
pub const MAX_PROCESS_ENVIRONMENT_BYTES: usize = 64 * 1_024;
pub const MAX_PROCESS_ENVIRONMENT_RAW_BYTES: usize = MAX_SINGLE_PROCESS_ARGV_BYTES;
pub const PROCESS_ENVIRONMENT_READ_SENTINEL_BYTES: usize = 1;
pub const MAX_PROCESS_STAT_BYTES: usize = 64 * 1_024;
pub const PROCESS_STAT_READ_SENTINEL_BYTES: usize = 1;
pub const MAX_PROCESS_PATH_BYTES: usize = 4 * 1_024 * 1_024;
pub const MAX_PROCESS_FILE_DESCRIPTORS: usize = 4_096;
pub const MAX_PROCESS_EVIDENCE_PATHS: usize = 128;
#[cfg(any(target_os = "macos", target_os = "linux"))]
pub const PROCESS_FD_SENTINEL_COUNT: usize = 1;
pub const MAX_EVIDENCE_DEPTH: usize = 8;
pub const MAX_EVIDENCE_FILES: usize = 10_000;
pub const MAX_EVIDENCE_SCAN_ENTRIES: usize = 50_000;
pub const MAX_EVIDENCE_PENDING_DIRECTORIES: usize = MAX_EVIDENCE_FILES;
pub const EVIDENCE_HEAD_BYTES: u64 = 64 * 1024;
pub const EVIDENCE_TAIL_BYTES: u64 = 256 * 1024;
pub const DETECTION_INTERVAL: Duration = Duration::from_secs(2);
pub const MISSED_DETECTION_THRESHOLD: u8 = 3;
pub const INITIAL_INPUT_DELAY: Duration = Duration::from_millis(150);
pub const STATUS_UNKNOWN_EVENT: &str = "unknown";
pub const AUTO_RESTORE_FAILED_REASON: &str = "auto_restore_failed";
pub const NODE_ENTRYPOINT_ARG_INDEX: usize = 1;
/// Node launchers may place options such as `--max-old-space-size` before the
/// entrypoint, so the entrypoint is located within a bounded leading window.
pub const NODE_ENTRYPOINT_SCAN_LIMIT: usize = 8;

#[cfg(target_os = "macos")]
pub mod macos {
    pub const VNODE_PATH_INFO_FLAVOR: i32 = 2;
}

pub mod event {
    pub const BINDING_CHANGED: &str = "agent_binding_changed";
    pub const STATUS_CHANGED: &str = "agent_status_changed";
}

pub mod error {
    pub const BINDING_INVALID: &str = "agent_binding_invalid";
    pub const DETECTION_FAILED: &str = "agent_detection_failed";
    pub const PROCESS_SCAN_FAILED: &str = "agent_process_scan_failed";
    pub const BINDING_WRITE_FAILED: &str = "agent_binding_write_failed";
}

pub use yolo_agent_adapter_contract::constants::message as validation;

pub mod recovery {
    pub const AUTHORITY_CHANGED: &str = "Claude recovery authority changed during publication";
    pub const BINDING_MISSING: &str = "Claude recovery authority has no binding payload";
    pub const BINDING_PARENT_MISSING: &str = "Claude recovery binding has no session directory";
    pub const EVIDENCE_CHANGED: &str = "Claude recovery transcript evidence changed";
    pub const EVIDENCE_MISSING: &str = "Claude recovery transcript evidence is unavailable";
    pub const OUTBOX_AGENT_MISMATCH: &str = "Claude recovery outbox belongs to another process";
    pub const TRANSCRIPT_MISSING: &str = "Claude recovery authority has no transcript path";
}

pub mod process_error {
    pub const EVIDENCE_PATH_LIMIT: &str = "process evidence path inventory exceeds its budget";
    pub const FD_BUFFER_OVERFLOW: &str = "process file descriptor buffer overflows";
    pub const FD_BUFFER_RANGE: &str = "process file descriptor buffer exceeds libproc range";
    pub const FD_INVENTORY_FAILED: &str = "process file descriptor inventory failed";
    pub const FD_INVENTORY_LIMIT: &str = "process file descriptor inventory exceeds its budget";
    pub const FD_INVENTORY_MISALIGNED: &str = "misaligned process file descriptor response";
    pub const PID_INVALID: &str = "process id is outside the platform range";
    pub const PLATFORM_UNSUPPORTED: &str =
        "agent process observation is unsupported on this platform";
    pub const SNAPSHOT_INVALID: &str = "fenced process snapshot does not match its request";
    pub const VNODE_TYPE_INVALID: &str = "platform vnode descriptor type is invalid";
}

pub mod process_environment_error {
    pub const DUPLICATE_KEY: &str = "process environment contains a duplicate selected key";
    pub const FIELD_LIMIT: &str = "process environment field limit exceeded";
    pub const KEY_LIMIT: &str = "process environment key limit exceeded";
    pub const MALFORMED: &str = "process environment contains a malformed field";
    pub const RAW_LIMIT: &str = "raw process environment exceeds its budget";
    pub const READ: &str = "read process environment";
    pub const SELECTED_LIMIT: &str = "selected process environment exceeds its budget";
    pub const UNTERMINATED: &str = "process environment is not terminated";
    pub const VALUE_LIMIT: &str = "process environment value limit exceeded";
    pub const VALUE_UTF8: &str = "process environment value is not UTF-8";
}

pub mod process_argv_error {
    pub const RAW_LIMIT: &str = "raw process argv exceeds its budget";
    pub const READ: &str = "read process argv";
    pub const UNTERMINATED: &str = "process argv is not terminated";
}

pub mod process_inventory_error {
    pub const DUPLICATE_PID: &str = "process inventory contains a duplicate process id";
    pub const MALFORMED_PID: &str = "process inventory contains a malformed process id";
    pub const OPEN: &str = "open process inventory";
    pub const PID_LIMIT: &str = "process inventory process id limit exceeded";
    pub const READ: &str = "read process inventory entry";
}

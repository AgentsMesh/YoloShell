pub const SOURCE_DIRECTORY: &str = "bundle-source";
pub const ENTRY_STATE: &str = "entry-hub";
pub const REMOTE_INSTALL: &str = "remote-install";
pub const REMOTE_STATE: &str = "remote-hub";
pub const REMOTE_HOME: &str = "remote-home";
pub const SESSION_NAME: &str = "cold SSH setup shell";
pub const SHELL_OUTPUT_PREFIX: &str = "YOLO_COLD_SSH_";
pub const SHELL_OUTPUT_COMMAND: &str = "printf 'YOLO_COLD_SSH_%s\\n'";
pub const POLL: std::time::Duration = std::time::Duration::from_millis(100);
pub const CLEANUP_MARGIN: std::time::Duration = std::time::Duration::from_secs(30);
pub const SETUP_FAILED: &str = "cold SSH setup did not produce a linked result";
pub const MAX_JOURNAL_BYTES: u64 = 2_097_152;
pub const BROKER_JOURNAL: &str = "hub-provisioning";
pub const ENROLLMENT_JOURNAL: &str = "enrollments";
pub const SETUP_JOURNAL: &str = "remote-setup";
pub const RECORD_FILE: &str = "record.json";
pub const JOURNAL_RECORD: &str = "record";
pub const JOURNAL_COMPLETION: &str = "completion";
pub const REEXPORTS: u8 = 1;

pub fn deadline() -> tokio::time::Instant {
    tokio::time::Instant::now()
        + yolo_remote_provisioning_protocol::PROVISION_OPERATION_TIMEOUT
        + CLEANUP_MARGIN
}

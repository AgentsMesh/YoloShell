pub const EVIDENCE_HEAD_BYTES: u64 = 64 * 1024;
pub const EVIDENCE_TAIL_BYTES: u64 = 256 * 1024;
pub const EXPECTED_FILE_LINKS: u64 = 1;

pub mod message {
    pub const DATA_HOME_NUL: &str = "provider data home contains a NUL byte";
    pub const DATA_HOME_INSPECT: &str = "inspect provider data home";
    pub const EVIDENCE_OPEN: &str = "open provider evidence";
    pub const EVIDENCE_INSPECT: &str = "inspect provider evidence";
    pub const EVIDENCE_SEEK: &str = "seek provider evidence";
    pub const EVIDENCE_READ: &str = "read provider evidence";
}

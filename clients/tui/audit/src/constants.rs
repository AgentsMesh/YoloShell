use std::time::Duration;

pub const DEFAULT_AUDIT_LIMIT: u16 = 128;
pub const MAX_AUDIT_LIMIT: u16 = 256;
pub const NO_LOCAL_HUB: &str = "No local Hub has been started; run `yolo server start` first";
pub const AUDIT_MERGE_MAX_SOURCES: usize = 16;
pub const AUDIT_MERGE_MAX_BYTES: u64 = 8 * 1024 * 1024;
pub const AUDIT_AFTER_WITH_MERGE: &str = "audit --after cannot be combined with --merge";
pub const AUDIT_IDENTITY_EXCLUSIVE: &str =
    "audit --identity cannot be combined with --merge, --after or --pull";
pub const AUDIT_MERGE_UNAUTHENTICATED: &str =
    "audit merge contains unsigned, invalid or untrusted records";
pub const AUDIT_SOURCE_FIELD: &str = "source";
pub const AUDIT_MERGE_FILE: &str = "audit merge sources must be regular files";
pub const AUDIT_PULL_TIMEOUT: Duration = Duration::from_secs(15);
pub const AUDIT_PULL_TRUST_REQUIRES_OUTPUT: &str = "audit pull with --trust requires --output";
pub const AUDIT_PULL_UNEXPECTED_REPLY: &str = "Hub returned an unexpected audit reply";
pub const AUDIT_PULL_WRITE_FAILED: &str = "write the pulled audit file";

pub const STARTUP_CHECKPOINT_SCHEMA_VERSION: u16 = 8;
pub const MAX_PROCESS_ID: u32 = i32::MAX as u32;
pub const SIZE_SENTINEL_BYTES: u64 = 1;
pub const LAUNCH_DIGEST_DOMAIN: &str = "yolo.worker-launch.v1";
pub const PREDECESSOR_RUNTIME_DIGEST_DOMAIN: &str = "yolo.predecessor-runtime.v1";
pub const CHECKPOINT_NAMESPACE_LABEL: &str = "worker startup checkpoint session";

pub const SCHEMA_ERROR: &str = "unsupported worker startup checkpoint schema";
pub const IDENTITY_ERROR: &str = "worker startup checkpoint launch identity is invalid";
pub const PROCESS_ERROR: &str = "worker startup checkpoint process identity is invalid";
pub const BOOT_ID_ERROR: &str = "worker startup checkpoint boot identity is invalid";
pub const CHILD_ERROR: &str = "worker startup checkpoint child binding is invalid";
pub const PROGRESS_ERROR: &str = "worker startup checkpoint progress is invalid";
pub const SUCCESSOR_ERROR: &str = "worker startup checkpoint successor fence is invalid";
pub const TRANSITION_ERROR: &str = "worker startup checkpoint transition is invalid";
pub const READ_CONTEXT: &str = "read bounded worker startup checkpoint";
pub const PARSE_CONTEXT: &str = "parse worker startup checkpoint";
pub const CHANGED_ERROR: &str = "worker startup checkpoint changed while reading";
pub const STALE_ERROR: &str = "worker startup checkpoint no longer matches its launch attempt";
pub const EVIDENCE_MISSING_ERROR: &str = "worker startup checkpoint evidence is missing";
pub const EVIDENCE_STALE_ERROR: &str = "worker startup checkpoint settlement evidence is stale";
pub const EVIDENCE_PRESENT_ERROR: &str =
    "worker startup checkpoint must be absent before launch publication";
pub const PROCESS_LIVE_ERROR: &str = "worker startup checkpoint process custody is not settled";
pub const CHILD_CUSTODY_AMBIGUOUS_ERROR: &str =
    "worker startup checkpoint child custody is ambiguous";
pub const PREDECESSOR_ERROR: &str = "worker startup checkpoint predecessor evidence does not match";
pub const PREDECESSOR_PUBLICATION_ERROR: &str =
    "runtime publication predecessor authority no longer matches startup evidence";
pub const PREDECESSOR_CUSTODY_ERROR: &str =
    "PTY custody predecessor authority no longer matches startup evidence";
pub const LIFECYCLE_BUSY_ERROR: &str = "worker startup checkpoint lifecycle lock is busy";

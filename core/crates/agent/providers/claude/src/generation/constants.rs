pub(crate) const AUTHORITY_SCHEMA_VERSION: u32 = 6;
pub(super) const AUTHORITY_SET_SCHEMA_VERSION: u32 = 1;
pub(super) const AUTHORITY_FILE_NAME: &str = "claude-generation.json";
pub(super) const INITIAL_GENERATION: u64 = 1;
pub(super) const INITIAL_REVISION: u64 = 1;
pub(super) const INITIAL_EVENT_SEQUENCE: u64 = 0;
pub(super) const INITIAL_PUBLICATION_GENERATION: u64 = 1;
pub(super) const REVISION_INCREMENT: u64 = 1;
pub(super) const GENERATION_INCREMENT: u64 = 1;
pub(super) const MAX_REPLAY_EVENT_IDS: usize = 64;
pub(super) const MAX_INVOCATION_AUTHORITIES: usize = 64;
#[cfg(test)]
pub(super) const AUTHORITY_REVISION_FIELD: &str = "authority_revision";
#[cfg(test)]
pub(super) const DURABLE_STATE_LOCK_RETRY_MILLIS: u64 = 1;
#[cfg(test)]
pub(super) const DURABLE_STATE_LOCK_TIMEOUT_SECS: u64 = 8;
#[cfg(test)]
pub(super) const DURABLE_STATE_LOCK_RETRY: std::time::Duration =
    std::time::Duration::from_millis(DURABLE_STATE_LOCK_RETRY_MILLIS);
#[cfg(test)]
pub(super) const DURABLE_STATE_LOCK_TIMEOUT: std::time::Duration =
    std::time::Duration::from_secs(DURABLE_STATE_LOCK_TIMEOUT_SECS);
pub(super) const TRANSCRIPT_RELATIVE_COMPONENTS: usize = 3;
pub(super) const BINDING_PARENT_MISSING: &str = "agent binding path has no session directory";
pub(super) const AUTHORITY_MISSING: &str = "Claude generation authority is missing";
pub(super) const AUTHORITY_AUTHENTICATION_MISMATCH: &str =
    "Claude generation authority authentication mismatch";
pub(super) const AUTHORITY_SESSION_MISMATCH: &str =
    "Claude generation authority belongs to another Yolo session";
pub(super) const AUTHORITY_SCHEMA_MISMATCH: &str = "unsupported Claude generation authority schema";
pub(super) const AUTHORITY_REVISION_OVERFLOW: &str =
    "Claude generation authority revision overflow";
pub(super) const AUTHORITY_GENERATION_OVERFLOW: &str =
    "Claude generation authority generation overflow";
pub(super) const AUTHORITY_PUBLICATION_GENERATION_OVERFLOW: &str =
    "Claude authority publication generation overflow";
pub(super) const AUTHORITY_BEGIN_INVALID: &str = "invalid Claude generation begin request";
pub(super) const AUTHORITY_EVENT_INVALID: &str = "invalid Claude generation event";
pub(super) const AUTHORITY_TRANSCRIPT_IDENTITY_MISSING: &str =
    "Claude generation transcript identity is missing";
pub(super) const AUTHORITY_TRANSCRIPT_IDENTITY_MISMATCH: &str =
    "Claude generation transcript identity changed";
pub(super) const PROCESS_EXIT_IDENTITY_MISMATCH: &str =
    "Claude process exit identity does not match generation authority";
pub(super) const PROCESS_EXIT_NOT_FINAL: &str =
    "Claude process exit has no final SessionEnd candidate";
pub(super) const PROCESS_EXIT_PROOF_UNAVAILABLE: &str =
    "Claude process exit is not proven by the operating system";
pub(super) const AUTHORITY_LIVE_RESERVATION: &str =
    "a live Claude invocation already owns this terminal runtime";
pub(super) const AUTHORITY_SET_INVALID: &str = "invalid Claude invocation authority set";
pub(super) const AUTHORITY_SET_CAPACITY: &str = "Claude invocation authority capacity is exhausted";
pub(super) const AUTHORITY_FOREGROUND_AMBIGUOUS: &str =
    "multiple Claude invocation authorities match the foreground process";
pub(super) const AUTHORITY_LOAD_CONTEXT: &str = "load Claude generation authority";
pub(super) const AUTHORITY_CREATE_CONTEXT: &str = "create Claude generation authority";
pub(super) const AUTHORITY_COMPARE_EXCHANGE_CONTEXT: &str =
    "compare-exchange Claude generation authority";
pub(super) const AUTHORITY_SNAPSHOT_MISMATCH: &str =
    "Claude generation authority changed during evidence validation";
pub(super) const AUTHORITY_PUBLICATION_STATE_INVALID: &str =
    "Claude generation binding publication state is invalid";
pub(super) const AUTHORITY_RECOVERY_INVALID: &str =
    "Claude generation recovery events do not match durable authority";
pub(super) const AUTHORITY_PARKED_RECOVERY_INVALID: &str =
    "Claude parked invocation is not eligible for recovery";
#[cfg(test)]
pub(super) const DURABLE_STATE_LOCK_CONTEXT: &str = "acquire Claude durable binding lifecycle lock";
pub(super) const GENERATION_MANIFEST_CONTEXT: &str = "read Claude generation session manifest";
pub(super) const GENERATION_MANIFEST_INVALID: &str =
    "Claude generation requires an active current session manifest";
pub(super) const GENERATION_TARGET_CHANGED: &str =
    "Claude generation target changed during commit fence";
pub(super) const GENERATION_PATH_INVALID: &str =
    "Claude generation path is not a managed Yolo session";
pub(super) const DURABLE_BINDING_MISSING: &str = "Claude durable binding is missing";
pub(super) const DURABLE_PENDING_INVALID: &str =
    "Claude durable binding pending transition is invalid";
pub(super) const DURABLE_BINDING_INVALID: &str = "Claude durable binding does not match authority";
pub(super) const DURABLE_PUBLICATION_INVALID: &str =
    "Claude durable binding publication is invalid";

use std::time::Duration;

pub use yolo_codex_hook_contract::*;

pub use super::outbox_budget::{MAX_ACKNOWLEDGED_EVENTS, MAX_OUTBOX_BYTES, MAX_PENDING_EVENTS};
pub(crate) use super::outbox_budget::{
    MAX_DURABLE_REJECTION_BYTES, MAX_DURABLE_REJECTIONS, MAX_OCCURRENCE_BINDING_BYTES,
    MAX_OCCURRENCE_BINDINGS,
};

pub const ACK_SCHEMA_VERSION: u16 = 1;
pub const ACK_CODE_ACCEPTED: &str = "accepted";
pub const ACK_CODE_REPLAY: &str = "replay";
pub const ACK_CODE_INVALID: &str = "invalid";
pub const ACK_CODE_IDENTITY_MISMATCH: &str = "identity_mismatch";
pub const ACK_CODE_SEQUENCE_MISMATCH: &str = "sequence_mismatch";
pub const ACK_CODE_AUTHORITY_REVOKED: &str = "authority_revoked";
pub const ACK_CODE_PROCESS_UNAVAILABLE: &str = "process_unavailable";
pub const ACK_CODE_PROTOCOL_ERROR: &str = "protocol_error";
pub const PROVIDER_VERSION_SEPARATOR: char = '.';
pub const FIRST_SEQUENCE: u64 = 1;
pub const MAX_INPUT_BYTES: usize = 256 * 1024;
pub(crate) const MAX_SNAPSHOT_BYTES: u64 = crate::paths::MAX_DURABLE_JSON_BYTES;
pub const FRAME_HEADER_BYTES: usize = std::mem::size_of::<u32>();
pub const MAX_ACK_BYTES: usize = 2 * 1024;
pub const MAX_DELIVERY_FRAME_BYTES: usize = MAX_OUTBOX_BYTES;
pub const LOCK_RETRY: Duration = Duration::from_millis(5);
pub const LOCK_TIMEOUT: Duration = Duration::from_secs(8);
pub const DELIVERY_TIMEOUT: Duration = crate::session_timing::RUST_INSPECTION_CLIENT_TIMEOUT;
pub const CODEX_MANAGEMENT_HANDSHAKE_TIMEOUT: Duration = DELIVERY_TIMEOUT;
pub const RETIREMENT_WRITER_TIMEOUT: Duration = LOCK_TIMEOUT;
pub const DELIVERY_RETRY_DELAY: Duration = Duration::from_millis(50);
pub const LOCK_RECORD_MAX_BYTES: u64 = 0;

pub const OUTBOX_LOCK_SUFFIX: &str = ".lock";
pub(crate) const SNAPSHOT_LOCK_SUFFIX: &str = OUTBOX_LOCK_SUFFIX;
pub const OUTBOX_SCHEMA_VERSION: u16 = 3;
pub const ARTIFACTS_DIRECTORY_NAME: &str = "codex-hooks";
pub const OUTBOX_FILE_NAME: &str = "outbox.json";
pub const SNAPSHOT_FILE_NAME: &str = "snapshot.json";

pub mod field_names {
    pub const SESSION_ID: &str = "session_id";
    pub const TRANSCRIPT_PATH: &str = "transcript_path";
    pub const CWD: &str = "cwd";
    pub const EVENT_NAME: &str = "hook_event_name";
    pub const TURN_ID: &str = "turn_id";
    pub const AGENT_ID: &str = "agent_id";
    pub const AGENT_TYPE: &str = "agent_type";
    pub const AGENT_TRANSCRIPT_PATH: &str = "agent_transcript_path";
    pub const MODEL: &str = "model";
    pub const PERMISSION_MODE: &str = "permission_mode";
    pub const PROMPT: &str = "prompt";
    pub const TOOL_NAME: &str = "tool_name";
    pub const TOOL_USE_ID: &str = "tool_use_id";
    pub const TOOL_INPUT: &str = "tool_input";
    pub const TOOL_RESPONSE: &str = "tool_response";
    pub const STOP_HOOK_ACTIVE: &str = "stop_hook_active";
    pub const LAST_ASSISTANT_MESSAGE: &str = "last_assistant_message";
    pub const SOURCE: &str = "source";
    pub const REASON: &str = "reason";
    pub const TRIGGER: &str = "trigger";
}

#[cfg(test)]
pub mod ack_field_names {
    pub const SCHEMA_VERSION: &str = "schema_version";
    pub const ACCEPTED: &str = "accepted";
    pub const EVENT_ID: &str = "event_id";
    pub const SEQUENCE: &str = "sequence";
    pub const CODE: &str = "code";
}

pub mod values {
    pub const STARTUP: &str = "startup";
    pub const RESUME: &str = "resume";
    pub const CLEAR: &str = "clear";
    pub const COMPACT: &str = "compact";
    pub const OTHER: &str = "other";
    pub const MANUAL: &str = "manual";
    pub const AUTO: &str = "auto";
    pub const START_SOURCES: &[&str] = &[STARTUP, RESUME, CLEAR, COMPACT];
    pub const COMPACT_TRIGGERS: &[&str] = &[MANUAL, AUTO];
}

pub mod permission_modes {
    pub const DEFAULT: &str = "default";
    pub const ACCEPT_EDITS: &str = "acceptEdits";
    pub const PLAN: &str = "plan";
    pub const DONT_ASK: &str = "dontAsk";
    pub const BYPASS_PERMISSIONS: &str = "bypassPermissions";
    pub const ALL: &[&str] = &[DEFAULT, ACCEPT_EDITS, PLAN, DONT_ASK, BYPASS_PERMISSIONS];
}

pub mod outbox_fields {
    pub const PENDING: &str = "pending";
}

#[cfg(any(test, feature = "runtime-component-tests"))]
#[path = "../test_support/wire_constants.rs"]
pub mod test_values;

#[path = "constants_errors.rs"]
pub mod errors;

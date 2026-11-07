pub use yolo_hooks::constants::*;

#[cfg(any(test, feature = "yolo-test-support"))]
pub mod test_values {
    pub const FABRICATED_SESSION: &str = "fabricated-session";
    pub const FORBIDDEN: &str = "forbidden";
    pub const IMPOSSIBLE_TOOL: &str = "impossible-tool";
    pub const NEXT_PROVIDER_SESSION: &str = "provider-session-after-clear";
    pub const RUNNING: &str = "running";
    pub const SESSION_START_KIND: &str = "session_start";
    pub const STARTUP_SOURCE: &str = "startup";
    pub const SUBAGENT_START_KIND: &str = "subagent_start";
    pub const TOOL_NAME: &str = "test_tool";
    pub const TOOL_USE_ID: &str = "test_tool_use";
    pub const UNDOCUMENTED_INTERRUPT_EVENT: &str = "Interrupt";
    pub const UNKNOWN: &str = "unknown";
}

#[cfg(test)]
pub mod snapshot_fields {
    pub const AGENT_ID: &str = "agent_id";
    pub const AUTHORITY_REVISION: &str = "authority_revision";
    pub const BINDING: &str = "binding";
    pub const CHILD_COMPACTIONS: &str = "child_compactions";
    pub const CHILDREN: &str = "children";
    pub const COMPACTION: &str = "compaction";
    pub const CWD: &str = "cwd";
    pub const ENTRIES: &str = "entries";
    pub const ENVELOPE_SHA256: &str = "envelope_sha256";
    pub const EVENT: &str = "event";
    pub const EVENT_ID: &str = "event_id";
    pub const EVENT_IDS: &str = "event_ids";
    pub const HISTORY: &str = "history";
    pub const IDENTITY: &str = "identity";
    pub const KIND: &str = "kind";
    pub const LAST_SEQUENCE: &str = "last_sequence";
    pub const LIFECYCLE: &str = "lifecycle";
    pub const METADATA: &str = "metadata";
    pub const NOTIFICATION: &str = "notification";
    pub const OBSERVATION_SHA256: &str = "observation_sha256";
    pub const PHASE: &str = "phase";
    pub const PID: &str = "pid";
    pub const PREDECESSOR: &str = "predecessor";
    pub const PREDECESSOR_SHA256: &str = "predecessor_sha256";
    pub const PRIOR_TURN: &str = "prior_turn";
    pub const PROCESS: &str = "process";
    pub const PROVIDER: &str = "provider";
    pub const PROVIDER_VERSION: &str = "provider_version";
    pub const REVOCATION: &str = "revocation";
    pub const RUNTIME_EPOCH: &str = "runtime_epoch";
    pub const SCHEMA_VERSION: &str = "schema_version";
    pub const SEQUENCE: &str = "sequence";
    pub const SOURCE: &str = "source";
    pub const START_TIME: &str = "start_time";
    pub const STARTED_AT: &str = "started_at";
    pub const STOP_CANDIDATE: &str = "stop_candidate";
    pub const TURN: &str = "turn";
    pub const UNKNOWN: &str = "unknown";
    pub const VIOLATION: &str = "violation";
}

#[cfg(test)]
pub mod snapshot_indexes {
    pub const FIRST: &str = "0";
    pub const SECOND: &str = "1";
    pub const NOTIFICATION: &str = "5";
}

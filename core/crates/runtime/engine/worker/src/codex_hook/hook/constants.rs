use std::time::Duration;

pub(super) const CODEX_SOCKET_PROBE_TIMEOUT: Duration = Duration::from_millis(250);
pub(super) const CODEX_REQUEST_TIMEOUT: Duration =
    crate::session_timing::AGENT_HOOK_REQUEST_TIMEOUT;
pub(super) const CODEX_RESPONSE_TIMEOUT: Duration =
    crate::session_timing::AGENT_HOOK_RESPONSE_TIMEOUT;
pub(super) const CODEX_MAX_CLIENTS: usize = 64;
pub(super) const CODEX_INGRESS_ACCEPT_RETRY_LIMIT: u32 = 8;
pub(super) const CODEX_INGRESS_UNSETTLED_PERMITS: usize = 0;
pub(super) const CODEX_INGRESS_RESTART_FAILURE_INCREMENT: u32 = 1;
pub(super) const CODEX_INGRESS_RESTART_INITIAL_DELAY: Duration =
    crate::worker::supervision::SUPERVISION_RESTART_INITIAL_DELAY;
pub(super) const CODEX_INGRESS_RESTART_MAX_DELAY: Duration =
    crate::worker::supervision::SUPERVISION_RESTART_MAX_DELAY;
pub(super) const CODEX_INGRESS_ABORT_TIMEOUT: Duration =
    crate::session_timing::AGENT_HOOK_ABORT_TIMEOUT;
pub(super) const CODEX_INGRESS_CONNECTION_DRAIN_TIMEOUT: Duration = CODEX_REQUEST_TIMEOUT
    .saturating_add(CODEX_RESPONSE_TIMEOUT)
    .saturating_add(CODEX_INGRESS_ABORT_TIMEOUT);
pub(super) const CODEX_INGRESS_BLOCKING_DRAIN_TIMEOUT: Duration =
    CODEX_INGRESS_CONNECTION_DRAIN_TIMEOUT;
pub(super) const CODEX_INGRESS_DRAIN_TIMEOUT: Duration =
    crate::session_timing::AGENT_HOOK_DRAIN_TIMEOUT;
pub(super) const CODEX_PROJECTION_DRAIN_TIMEOUT: Duration = CODEX_INGRESS_DRAIN_TIMEOUT;
pub(super) const CODEX_PROJECTION_DRAIN_ERROR: &str = "Codex terminal projection did not drain";
pub(super) const CODEX_PROJECTION_COMPLETION_CLOSED: &str = "Codex projection completion was lost";
pub(super) const CODEX_PROJECTION_COMPLETION_POISONED: &str =
    "Codex projection completion is poisoned";
pub(super) const CODEX_PROJECTION_STARTED: &str = "Codex projection already started";
pub(super) const CODEX_INTAKE_RETRY_INTERVAL: Duration = Duration::from_millis(100);
pub(super) const CODEX_INGRESS_ACCEPT_RETRY_ERROR: &str =
    "Codex hook listener retry budget is exhausted";
pub(super) const CODEX_INGRESS_TASK_ERROR: &str = "Codex hook ingress task stopped unexpectedly";
pub(super) const CODEX_INGRESS_TASK_NAME: &str = "codex_hook_ingress";
pub(super) const CODEX_INGRESS_ALREADY_RUNNING_ERROR: &str =
    "Codex hook ingress is already running for this Worker";
pub(super) const CODEX_INGRESS_START_CANCELLED_ERROR: &str =
    "Codex hook ingress startup was cancelled";
pub(super) const CODEX_INGRESS_START_RESULT_ERROR: &str =
    "Codex hook ingress startup result was lost";
pub(super) const CODEX_INGRESS_LISTENER_ABORTED_ERROR: &str =
    "Codex hook listener task was aborted unexpectedly";
pub(super) const CODEX_PROJECTION_ROUTER_UNAVAILABLE: &str =
    "Codex authority projection router is unavailable";
pub(super) const CODEX_PROJECTION_CHANNEL_CAPACITY: usize =
    super::router::MAX_ACTIVE_CODEX_INVOCATIONS;
pub(super) const CODEX_PROJECTION_ROUTE_LAG: &str =
    "Codex authority projection missed a committed route";
pub(super) const CODEX_PROJECTION_ROUTE_CLOSED: &str =
    "Codex authority projection route stream closed";
pub(super) const CODEX_PROJECTION_FORWARDER_JOIN: &str =
    "Codex authority projection forwarder failed";
pub(super) const CODEX_PROJECTION_VERSION_CONFLICT: &str =
    "Codex authority projection version is inconsistent";
pub(super) const CODEX_PROJECTION_BINDING_CHANGED: &str =
    "Codex projected binding changed after authority publication";
pub(super) const CODEX_INGRESS_CONNECTION_ERROR: &str =
    "Codex hook connection task failed and was contained";
pub(super) const CODEX_INGRESS_CONNECTION_DRAIN_ERROR: &str =
    "Codex hook connections exceeded their graceful drain deadline";
pub(super) const CODEX_INGRESS_BLOCKING_DRAIN_ERROR: &str =
    "Codex hook blocking settlements exceeded their drain deadline";
pub(super) const CODEX_INGRESS_SHUTDOWN_ERROR: &str =
    "Codex hook ingress task did not stop before its deadline";
pub(super) const CODEX_INGRESS_LISTENER_DRAIN_ERROR: &str =
    "Codex hook listener did not withdraw before its abort deadline";
pub(super) const CODEX_INGRESS_GENERATION_REVOKED_ERROR: &str =
    "Codex hook ingress generation was revoked before response publication";
pub(super) const CODEX_INTAKE_ENUMERATION_ERROR: &str =
    "Codex committed intake authority enumeration failed";
pub(super) const CODEX_INTAKE_ROUTE_ERROR_LOG: &str =
    "Codex durable intake route reconciliation failed";
pub(super) const CODEX_INTAKE_ROUND_LOG: &str =
    "Codex durable intake reconciliation round completed";
pub(super) const CODEX_INTAKE_TASK_ERROR: &str = "Codex durable intake reconciliation task failed";
pub(super) const CODEX_FRAME_HEADER_BYTES: usize = std::mem::size_of::<u32>();
pub(super) const CODEX_MIN_FRAME_BYTES: usize = 1;
pub(super) const CODEX_MAX_FRAME_BYTES: usize = crate::codex_hooks::MAX_OUTBOX_BYTES;
pub(super) const CODEX_ACK_INVALID: &str = crate::codex_hooks::ACK_CODE_INVALID;
pub(super) const CODEX_ACK_IDENTITY: &str = crate::codex_hooks::ACK_CODE_IDENTITY_MISMATCH;
pub(super) const CODEX_ACK_SEQUENCE: &str = crate::codex_hooks::ACK_CODE_SEQUENCE_MISMATCH;
pub(super) const CODEX_ACK_REVOKED: &str = crate::codex_hooks::ACK_CODE_AUTHORITY_REVOKED;
pub(super) const CODEX_ACK_PROCESS: &str = crate::codex_hooks::ACK_CODE_PROCESS_UNAVAILABLE;
pub(super) const CODEX_ACK_PROTOCOL: &str = crate::codex_hooks::ACK_CODE_PROTOCOL_ERROR;
pub(super) const CODEX_PEER_UID_ERROR: &str = "Codex hook peer user does not match this Worker";
pub(super) const CODEX_PEER_PID_ERROR: &str = "Codex hook peer process identity is unavailable";
pub(super) const CODEX_SOCKET_PARENT_ERROR: &str = "Codex hook socket has no parent";
pub(super) const CODEX_SOCKET_LIVE_ERROR: &str = "Codex hook socket is already live";
pub(super) const CODEX_SOCKET_BIND_ERROR: &str = "bind retained Codex hook socket";
pub(super) const CODEX_SOCKET_NAMESPACE_ERROR: &str =
    "Codex hook socket namespace authority was lost";
pub(super) const CODEX_LISTENER_ERROR: &str = "Codex hook listener is unavailable";
pub(super) const CODEX_CLIENT_LIMIT_ERROR: &str = "Codex hook client capacity is exhausted";
pub(super) const CODEX_SESSION_ERROR: &str = "Codex hook session identity does not match Worker";
pub(super) const CODEX_RUNTIME_ERROR: &str = "Codex hook runtime identity does not match Worker";
pub(super) const CODEX_VERSION_ERROR: &str = "Codex hook provider version is not admitted";
pub(super) const CODEX_OWNER_ERROR: &str =
    "Codex hook invocation identity does not match authority";
pub(super) const CODEX_PROCESS_ERROR: &str = "Codex hook process identity is not admitted";
pub(super) const CODEX_RECOVERY_PROCESS_ERROR: &str = "Codex recovery process identity is unproven";
pub(super) const CODEX_ROUTE_RECOVERY_ERROR: &str = "Codex durable route recovery failed";
pub(super) const CODEX_RECOVERY_ROUND_ERROR: &str =
    "Codex durable recovery round authority was not retained";
pub(super) const CODEX_REPLAY_ERROR: &str = "Codex hook outbox replay failed";
pub(super) const CODEX_REDUCER_ERROR: &str = "Codex hook lifecycle reduction failed";
pub(super) const CODEX_STORAGE_ERROR: &str = "Codex hook authority storage failed";
pub(super) const CODEX_STORAGE_LATCHED_ERROR: &str =
    "Codex hook authority storage is permanently unavailable";
pub(super) const CODEX_COMMIT_INDETERMINATE_ERROR: &str =
    "Codex hook outbox commit durability remains indeterminate";
pub(super) const CODEX_AUTHORITY_EXHAUSTED_ERROR: &str =
    "Codex hook authority revision is exhausted";
pub(super) const CODEX_AUTHORITY_LOST_ERROR: &str = "Codex hook runtime authority was lost";
pub(super) const CODEX_AUTHORITY_SUPERSEDED_ERROR: &str =
    "Codex hook runtime authority was superseded";
pub(super) const CODEX_DURABLE_EVENT_ERROR: &str =
    "Codex hook event is not the durable outbox authority";
pub(super) const CODEX_OUTBOX_OWNER_MISSING_ERROR: &str =
    "Codex hook durable outbox owner is missing";
pub(super) const CODEX_OUTBOX_TIP_ERROR: &str =
    "Codex hook snapshot is ahead of the durable outbox";
pub(super) const CODEX_SNAPSHOT_WATERMARK_ERROR: &str =
    "Codex hook snapshot is behind the acknowledged outbox";
pub(super) const CODEX_OUTBOX_HISTORY_ERROR: &str =
    "Codex hook durable outbox history is incomplete or divergent";
pub(super) const CODEX_ACK_WRITE_ERROR: &str = "Codex hook acknowledgement write failed";
pub(super) const CODEX_REQUEST_READ_ERROR: &str = "Codex hook request read failed";
pub(super) const CODEX_REQUEST_TIMEOUT_ERROR: &str = "Codex hook request timed out";
pub(super) const CODEX_RESPONSE_TIMEOUT_ERROR: &str = "Codex hook acknowledgement timed out";
pub(super) const CODEX_HANDLER_JOIN_ERROR: &str = "Codex hook ingestion task failed";
pub(super) const CODEX_REJECTION_LOG: &str = "rejected Codex hook lifecycle event";
pub(super) const CODEX_PROTOCOL_REJECTION_LOG: &str = "rejected Codex hook protocol request";

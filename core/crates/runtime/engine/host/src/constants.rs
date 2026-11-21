use std::time::Duration;

pub(super) const SOCKET_PROBE_TIMEOUT: Duration = Duration::from_millis(250);
#[cfg(test)]
pub(super) const HOST_SOCKET_PERMISSION_MASK: u32 = 0o7777;
pub(super) const HOST_SOCKET_LIVE_CONTEXT: &str = "another Yolo host is already listening";
pub(super) const HOST_INGRESS_ARM_CONTEXT: &str = "host ingress stopped before listener polling";
pub(super) const HOST_INGRESS_JOIN_CONTEXT: &str = "host ingress task failed";
pub(super) const HOST_INGRESS_STOPPED_CONTEXT: &str = "host ingress stopped unexpectedly";
pub(super) const HOST_SERVICE_OUTCOME_CONTEXT: &str = "host service outcome failed";
pub(super) const HOST_SHUTDOWN_INGRESS_CONTEXT: &str = "host ingress shutdown failed";
pub(super) const HOST_SHUTDOWN_MAINTENANCE_CONTEXT: &str = "host maintenance shutdown failed";
pub(super) const HOST_SHUTDOWN_MULTIPLE_CONTEXT: &str = "multiple host shutdown stages failed";
pub(super) const SERVING_HOST_RELEASED: &str = "serving Host was already released";
pub(super) const SERVING_HOST_PUBLICATION_CAPTURED: &str =
    "serving Host publication was already captured";
#[cfg(test)]
pub(super) const HOST_SERVICE_TEST_ERROR: &str = "host service test failure";
#[cfg(test)]
pub(super) const HOST_INGRESS_TEST_ERROR: &str = "host ingress test failure";
#[cfg(test)]
pub(super) const HOST_MAINTENANCE_TEST_ERROR: &str = "host maintenance test failure";
#[cfg(test)]
pub(super) const HOST_SOCKET_PROBE_TIMEOUT_CONTEXT: &str = "host socket probe timed out";
#[cfg(test)]
pub(super) const HOST_SOCKET_PROBE_ERROR_CONTEXT: &str = "host socket probe was indeterminate";
#[cfg(test)]
pub(super) const HOST_SOCKET_CHANGED_CONTEXT: &str = "host socket path changed";
#[cfg(test)]
pub(super) const HOST_SOCKET_TYPE_ERROR: &str = "host IPC path is not a Unix socket";
#[cfg(test)]
pub(super) const HOST_SOCKET_OWNER_ERROR: &str = "host socket owner is unsafe";
#[cfg(test)]
pub(super) const HOST_SOCKET_PERMISSION_ERROR: &str = "host socket permissions are unsafe";
pub(super) const WORKER_READY_TIMEOUT: Duration = crate::session_timing::WORKER_READY_TIMEOUT;
pub(super) const WORKER_READY_POLL_INTERVAL: Duration = Duration::from_millis(25);
pub(super) const WORKER_STATUS_TIMEOUT: Duration = Duration::from_secs(1);
pub(super) use crate::session_timing::HOST_ARCHIVE_SETTLEMENT_TIMEOUT as ARCHIVE_SETTLEMENT_WAIT;
pub(super) use crate::session_timing::HOST_DEFERRED_TERMINATION_SETTLEMENT_TIMEOUT as DEFERRED_TERMINATION_SETTLEMENT_TIMEOUT;
pub(super) use crate::session_timing::HOST_PROCESS_EXIT_TIMEOUT as PROCESS_EXIT_TIMEOUT;
pub(super) const PROCESS_EXIT_POLL_INTERVAL: Duration = Duration::from_millis(20);
pub(super) const SESSION_NOT_FOUND: &str = "session does not exist";
pub(super) const ARCHIVE_FINALIZATION_PENDING: &str =
    "wait for session exit finalization before archiving it";
pub(super) const ARCHIVE_RUNTIME_UNSETTLED: &str =
    "session runtime lacks durable finalization evidence";
#[cfg(test)]
pub(super) const ARCHIVE_TEST_TIMEOUT: Duration = Duration::from_millis(250);
pub(super) const WORKER_STATUS_TIMEOUT_DETAIL: &str = "worker status request timed out";
pub(super) const WORKER_STATUS_FAILED: &str = "worker status failed";
pub(super) const WORKER_TERMINATION_FAILED: &str = "worker termination failed";
pub(super) const WORKER_ATTACHMENT_PROTOCOL_INVALID: &str =
    "worker returned an invalid attachment response";
pub(super) const WORKER_ATTACHMENT_RUNTIME_MISMATCH: &str =
    "worker attachment does not match the selected runtime";
pub(super) const WORKER_RETIREMENT_FAILED: &str = "worker retirement failed";
pub(super) const MAX_CATALOG_VIEW_CONCURRENCY: usize = 16;
pub(super) const WORKER_RETIREMENT_EXIT_TIMEOUT: &str =
    "retired worker did not exit before deadline";
pub(super) const WORKER_RETIREMENT_SOCKET_ACTIVE: &str =
    "retired worker socket is still accepting connections";
pub(super) const SESSION_EXIT_DEADLINE_EXCEEDED: &str =
    "session did not exit before the discard deadline";
pub(super) const WATCHDOG_INTERVAL: Duration = Duration::from_secs(2);
pub(super) const WATCHDOG_FAILURE_BACKOFF: Duration = Duration::from_secs(1);
pub(super) const MAX_RECONCILE_CONCURRENCY: usize = 8;
pub(super) const UNIX_SOCKET_PATH_MAX_BYTES: usize =
    crate::paths::constants::UNIX_SOCKET_PATH_MAX_BYTES;
pub(super) const WORKER_REAPER_CHANNEL_CAPACITY: usize = 1;
pub(super) const WORKER_REAPER_THREAD_PREFIX: &str = "yolo-worker-reap";
pub(super) const WORKER_BOOTSTRAP_FAILED: &str = "worker bootstrap delivered no input";
pub(super) const WORKER_BOOTSTRAP_UNKNOWN: &str = "worker bootstrap delivery is unknown";
pub(super) const WORKER_BOOTSTRAP_PENDING: &str = "worker bootstrap delivery is pending";
pub(super) const CREATION_LAUNCH_UNCERTAIN: &str =
    "reserved creation launch has not reached a durable terminal state";
pub(super) const CREATION_LAUNCH_FAILED: &str =
    "reserved creation worker bootstrap failed before becoming ready";
pub(super) const CREATION_RUNTIME_MISMATCH: &str =
    "runtime evidence does not match the reserved creation";
pub(super) const CREATION_SESSION_MISSING: &str =
    "incomplete creation is missing its durable session";
pub(super) const CREATION_PLAN_MISSING: &str = "incomplete creation has no planned identity";
pub(super) const CREATION_SETTLEMENT_INVARIANT: &str =
    "settlement returns only terminal creation evidence";
pub(super) const UNSTARTED_WORKER_LAUNCH_RETIRE_CONTEXT: &str =
    "retire worker launch after pre-spawn failure";
pub(super) const AGENT_CONTEXT_UNAVAILABLE: &str = "agent binding context is unavailable";
pub(super) const AGENT_BINDING_LOAD_FAILED: &str = "persisted agent binding is invalid";
pub(super) const DURABLE_STATE_CHANGED_CONTEXT: &str = "durable state changed while reading";
pub(super) const CREATION_LEDGER_FILE_NAME: &str = "creation-capabilities.json";
pub(super) const CREATION_LEDGER_LABEL: &str = "creation ledger";
pub(super) const CREATION_LEDGER_TEMP_SUFFIX: &str = ".tmp";
pub(super) const CREATION_LEDGER_SCHEMA_VERSION: u16 = 4;
pub(super) const MAX_CREATION_LEDGER_BYTES: u64 = 32 * 1024 * 1024;
pub(super) const MAX_CREATION_LEDGER_ENTRIES: usize = crate::constants::MAX_MANAGED_SESSIONS;
pub(super) const MAX_CREATION_LEDGER_PARENT_ENTRIES: usize = 4_096;
pub(super) const MAX_DURABLE_SESSION_DIRECTORY_ENTRIES: usize = MAX_CREATION_LEDGER_ENTRIES;
pub(super) const CREATION_PHASE_LAUNCH_INCONSISTENT: &str =
    "creation phase and launch identity are inconsistent";
pub(super) const CREATION_LAUNCH_PHASE_CONFLICT: &str =
    "creation phase cannot prepare a worker launch";
pub(super) const CREATION_PHASE_TRANSITION_CONFLICT: &str =
    "creation phase transition is not allowed";
pub(super) const SHA256_HEX_LENGTH: usize = 64;
pub(super) const WORKER_AUTH_TOKEN_HEX_LENGTH: usize = 64;
pub(super) const RESUME_TRANSACTION_MISSING: &str =
    "restoring session has no durable resume transaction";
pub(super) const RESUME_TRANSACTION_INVALID: &str = "durable resume transaction is invalid";
pub(super) const RESUME_TRANSACTION_CONFLICT: &str =
    "resume request conflicts with the durable recovery plan";
pub(super) const RESUME_TRANSACTION_RETRY_PARKED: &str =
    "durable resume retry requires new runtime evidence or an explicit retry";
pub(super) const RESUME_EVIDENCE_MISMATCH: &str =
    "runtime evidence does not match the durable resume transaction";
pub(super) const RESUME_PREVIOUS_RUNTIME_UNRESOLVED: &str =
    "previous runtime identities are not resolved";
pub(super) const RESUME_TARGET_SESSION_EMPTY: &str = "provider resume target has no session id";
pub(super) const RESUME_LAUNCH_DIGEST_INVALID: &str = "resume launch digest is invalid";
pub(super) const RESUME_LAUNCH_SERIALIZE_CONTEXT: &str = "serialize resume worker launch";
pub(super) const RESUME_TRANSACTION_SERIALIZE_CONTEXT: &str =
    "serialize durable resume transaction body";
pub(super) const RESUME_TRANSACTION_DIGEST_INVALID: &str =
    "durable resume transaction digest is invalid";
pub(super) const RESUME_LAUNCH_WITHOUT_TRANSACTION: &str =
    "worker launch exists without a durable resume transaction";
pub(super) const RESUME_AUTHENTICATED_RUNTIME_MISMATCH: &str =
    "authenticated worker identity does not match the recovery plan";
pub(super) const RESUME_MANIFEST_MISMATCH: &str =
    "durable session manifest conflicts with the recovery plan";
pub(super) const RESUME_TOMBSTONE_MISMATCH: &str =
    "termination tombstone does not match the resumed runtime epoch";
pub(super) const RESUME_BINDING_CHANGED: &str =
    "agent binding changed after the durable resume plan was published";
pub(super) const PUBLISHED_LAUNCH_SESSION_MISMATCH: &str =
    "published worker launch does not match its session";
pub(super) const RESUME_CREATION_ACTIVE: &str = "an unfinalized creation cannot be resumed";
pub(super) const RESUME_SESSION_RUNNING: &str = "session is already running";
pub(super) const RESUME_FINALIZATION_PENDING: &str =
    "session exit has not finished durable finalization";
pub(super) const RESUME_SOCKET_ACTIVE: &str =
    "worker socket is still serving and cannot be safely reused";
pub(super) const RESUME_RECONCILE_FAILED_LOG: &str =
    "durable resume transaction reconciliation failed";
#[cfg(any(test, feature = "failed-rearm-test-hooks"))]
#[path = "constants_resume_fixture.rs"]
mod resume_constants;
#[cfg(any(test, feature = "failed-rearm-test-hooks"))]
pub(super) use resume_constants::*;
pub(super) const MAX_CREATION_EVIDENCE_BYTES: u64 = 1024 * 1024;
pub(super) const CREATION_RECEIPT_STORE_DIRECTORY: &str = "creation-receipts";
pub(super) const CREATION_RECEIPT_LABEL: &str = "creation receipt";
pub(super) const CREATION_RECEIPT_INDEX_LABEL: &str = "creation receipt index";
pub(super) const CREATION_RECEIPT_DIRECTORY_LABEL: &str = "creation receipt directory";
pub(super) const CREATION_RECEIPT_TOKEN_DIRECTORY: &str = "tokens";
pub(super) const CREATION_RECEIPT_SESSION_DIRECTORY: &str = "sessions";
pub(super) const CREATION_RECEIPT_STAGING_DIRECTORY: &str = "staging";
pub(super) const CREATION_RECEIPT_SCHEMA_VERSION: u16 = 3;
pub(super) const CREATION_RECEIPT_FILE_MODE: u32 = 0o400;
pub(super) const CREATION_RECEIPT_TEMP_MODE: u32 = 0o600;
pub(super) const CREATION_RECEIPT_DIRECTORY_MODE: u32 = 0o700;
pub(super) const MAX_CREATION_RECEIPT_BYTES: u64 = 1024;
pub(super) const MAX_CREATION_RECEIPT_STAGING_ENTRIES: usize = 1024;
pub(super) const CREATION_RECEIPT_TEMP_PREFIX: &str = ".receipt.";
pub(super) const CREATION_RECEIPT_TEMP_SUFFIX: &str = ".tmp";
pub(super) const CREATION_RECEIPT_SLOT_PREFIX: &str = ".receipt-retired-";
pub(super) const CREATION_RECEIPT_NAME_HASH_BYTES: usize = 16;
pub(super) const EXPECTED_CREATION_RECEIPT_LINKS: u64 = 1;
pub(super) const CURRENT_DIRECTORY_ENTRY_BYTES: &[u8] = b".";
pub(super) const PARENT_DIRECTORY_ENTRY_BYTES: &[u8] = b"..";
pub(super) const RESERVED_SOCKET_NAMESPACE_ERROR: &str =
    "a host socket inside Yolo's reserved socket root must be this home's default endpoint";
pub(super) const HOST_INGRESS_SHUTDOWN_CONTEXT: &str = "stop host ingress during shutdown";
pub(super) const HOST_CLIENT_JOIN_PANIC_LOG: &str = "host client task panicked while joining";
pub(super) const HOST_IDLE_SHUTDOWN_JOIN_PANIC_LOG: &str =
    "host idle shutdown task panicked while joining";
pub(super) const REQUEST_RUNTIME_EPOCH_MISSING: &str =
    "runtime-bound request omitted runtime_epoch";
pub(super) const REQUEST_RUNTIME_EPOCH_INVALID: &str =
    "runtime-bound request has an invalid runtime_epoch";
pub(super) const REQUEST_RUNTIME_EPOCH_MISMATCH: &str =
    "runtime-bound request targets a replaced runtime";
pub(super) const REQUEST_RUNTIME_SESSION_MISSING: &str = "runtime-bound request omitted terminal_id";
pub(super) const REQUEST_RUNTIME_SESSION_INVALID: &str =
    "runtime-bound request has an invalid terminal_id";
include!("constants_creation_deadline.rs");
pub(super) const ATTACHMENT_NESTED_METHOD_UNSUPPORTED: &str =
    "attached connections only accept detach or output acknowledgement";

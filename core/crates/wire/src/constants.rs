pub const PROTOCOL_VERSION: u16 = 8;
pub const FRAME_LENGTH_BYTES: usize = std::mem::size_of::<u32>();
pub const MAX_FRAME_BYTES: usize = 16 * 1024 * 1024;
pub const SERDE_ACTION_TAG: &str = "action";
pub const SERDE_KIND_TAG: &str = "kind";
pub const SERDE_OPERATION_TAG: &str = "operation";
pub const SERDE_STATE_TAG: &str = "state";

pub mod method {
    pub const HELLO: &str = "hello";
    pub const HOST_STATUS: &str = "host_status";
    pub const HOST_BUILD: &str = "host_build";
    pub const LIST_SESSIONS: &str = "list_sessions";
    pub const CREATE_SESSION: &str = "create_session";
    pub const QUERY_CREATION: &str = "query_creation";
    pub const WORKSPACE_INSPECT: &str = "workspace_inspect";
    pub const WORKTREE_CREATE: &str = "worktree_create";
    pub const WORKTREE_REMOVE: &str = "worktree_remove";
    pub const WORKTREE_LIST: &str = "worktree_list";
    pub const GET_SESSION: &str = "get_session";
    pub const ATTACH_SESSION: &str = "attach_session";
    pub const WATCH_AGENT_EVENTS: &str = "watch_agent_events";
    pub const WATCH_CATALOGUE: &str = "watch_catalogue";
    pub const GET_SNAPSHOT: &str = "get_snapshot";
    pub const GET_TEXT: &str = "get_text";
    pub const READ_RETAINED_HISTORY: &str = "read_retained_history";
    pub const TERMINAL_INPUT: &str = "terminal_input";
    pub const RESIZE_TERMINAL: &str = "resize_terminal";
    pub const SIGNAL_SESSION: &str = "signal_session";
    pub const OBSERVE_NATIVE_AGENT: &str = "observe_native_agent";
    pub const GET_NATIVE_CONTROL_RECEIPT: &str = "get_native_control_receipt";
    pub const INTERRUPT_NATIVE_AGENT: &str = "interrupt_native_agent";
    pub const SUBMIT_NATIVE_AGENT: &str = "submit_native_agent";
    pub const RESPOND_NATIVE_AGENT: &str = "respond_native_agent";
    pub const TERMINATE_SESSION: &str = "terminate_session";
    pub const TERMINATE_RUNTIME_EXACT: &str = "terminate_runtime_exact";
    pub const DISCARD_SESSION: &str = "discard_session";
    pub const CLEANUP_TERMINATE_SESSION: &str = "cleanup_terminate_session";
    pub const CLEANUP_ARCHIVE_SESSION: &str = "cleanup_archive_session";
    pub const CLEANUP_DISCARD_SESSION: &str = "cleanup_discard_session";
    pub const FINALIZE_CREATION: &str = "finalize_creation";
    pub const ACQUIRE_WRITER: &str = "acquire_writer";
    pub const RENEW_WRITER: &str = "renew_writer";
    pub const RELEASE_WRITER: &str = "release_writer";
    pub const TAKEOVER_WRITER: &str = "takeover_writer";
    pub const GET_WRITER_LEASE: &str = "get_writer_lease";
    pub const ARCHIVE_SESSION: &str = "archive_session";
    pub const RESUME_SESSION_EXACT: &str = "resume_session_exact";
    pub const RECONCILE: &str = "reconcile";
    pub const SHUTDOWN_IF_IDLE: &str = "shutdown_if_idle";
}

pub mod event {
    pub const SESSION_ADDED: &str = "session_added";
    pub const SESSION_UPDATED: &str = "session_updated";
    pub const SESSION_EXITED: &str = "session_exited";
    pub const SESSION_REMOVED: &str = "session_removed";
    pub const RECOVERY_CHANGED: &str = "recovery_changed";
    pub const TERMINAL_SNAPSHOT: &str = "terminal_snapshot";
    pub const TERMINAL_OUTPUT: &str = "terminal_output";
    pub const OUTPUT_GAP: &str = "output_gap";
    pub const WRITER_LEASE_ACQUIRED: &str = "writer_lease_acquired";
    pub const WRITER_LEASE_REVOKED: &str = "writer_lease_revoked";
    pub const WRITER_LEASE_EXPIRED: &str = "writer_lease_expired";
    pub const WRITER_LEASE_CONFLICTED: &str = "writer_lease_conflicted";
    pub const AGENT_BINDING_CHANGED: &str = "agent_binding_changed";
    pub const AGENT_STATUS_CHANGED: &str = "agent_status_changed";
    pub const CATALOGUE_CHANGED: &str = "catalogue_changed";
}

pub mod capability {
    pub const HOST_BUILD: &str = "host_build";
    pub const DURABLE_PTY: &str = "durable_pty";
    pub const HOST_ADOPTION: &str = "host_adoption";
    pub const CAPNP_BINARY: &str = "capnp_binary";
    pub const OUTPUT_JOURNAL: &str = "output_journal";
    pub const SAFE_PROCESS_TREE_TERMINATION: &str = "safe_process_tree_termination";
    pub const AGENT_RESUME: &str = "agent_resume";
    pub const NATIVE_AGENT_CONTROL: &str = "native_agent_control";
    pub const SSH_STDIO_BRIDGE: &str = "ssh_stdio_bridge";
    pub const ATTACH_BARRIER: &str = "attach_barrier";
    pub const SINGLE_WRITER_LEASE: &str = "single_writer_lease";
    pub const IDEMPOTENT_INPUT: &str = "idempotent_input";
    pub const PERSISTENT_JOURNAL: &str = "persistent_journal";
    pub const AUTO_RECONCILIATION: &str = "auto_reconciliation";
    pub const SHELL_AGENT_DETECTION: &str = "shell_agent_detection";
    pub const IDLE_HOST_SHUTDOWN: &str = "idle_host_shutdown";
    pub const WRITER_LEASE_V2: &str = "writer_lease_v2";
    pub const CREATION_CAPABILITY: &str = "creation_capability";
    pub const WORKSPACE_INSPECT: &str = "workspace_inspect";
    pub const WORKTREE: &str = "worktree";
}

pub mod error_code {
    pub const CONFLICT: &str = "conflict";
    pub const CLEANUP_DEFERRED: &str = "cleanup_deferred";
    pub const DELIVERY_UNKNOWN: &str = "delivery_unknown";
    pub const DESTRUCTIVE_SESSION_UNAVAILABLE: &str = "destructive_session_unavailable";
    pub const AGENT_RECOVERY_UNAVAILABLE: &str = "agent_recovery_unavailable";
    pub const HOST_REBOOTED: &str = "host_rebooted";
    pub const IDENTITY_MISMATCH: &str = "identity_mismatch";
    pub const INTERNAL: &str = "internal";
    pub const INVALID_REQUEST: &str = "invalid_request";
    pub const INVALID_DIRECTORY: &str = "invalid_directory";
    pub const LEASE_CONFLICT: &str = "lease_conflict";
    pub const NOT_FOUND: &str = "not_found";
    pub const PROTOCOL_INCOMPATIBLE: &str = "protocol_incompatible";
    pub const RECREATE_REQUIRED: &str = "recreate_required";
    pub const UNAUTHORIZED: &str = "unauthorized";
    pub const UNSUPPORTED_METHOD: &str = "unsupported_method";
    pub const WORKER_START_FAILED: &str = "worker_start_failed";
    pub const WORKER_UNREACHABLE: &str = "worker_unreachable";
    pub const WORKER_SHUTTING_DOWN: &str = "worker_shutting_down";
    pub const WORKTREE_UNAVAILABLE: &str = "worktree_unavailable";
    pub const WORKTREE_INVALID_REPOSITORY: &str = "worktree_invalid_repository";
    pub const WORKTREE_UNTRUSTED_REPOSITORY: &str = "worktree_untrusted_repository";
    pub const WORKTREE_INVALID_BRANCH: &str = "worktree_invalid_branch";
    pub const WORKTREE_TARGET_EXISTS: &str = "worktree_target_exists";
    pub const WORKTREE_NOT_REGISTERED: &str = "worktree_not_registered";
    pub const WORKTREE_GIT_FAILED: &str = "worktree_git_failed";
    pub const WORKTREE_TIMEOUT: &str = "worktree_timeout";
    pub const WORKTREE_OUTPUT_TOO_LARGE: &str = "worktree_output_too_large";
    pub const WORKTREE_UNKNOWN: &str = "worktree_unknown";
    pub const WORKTREE_IO_FAILED: &str = "worktree_io_failed";
}

#[path = "field_constants.rs"]
pub mod field;

pub mod delivery {
    pub const DELIVERED: &str = "delivered";
    pub const UNSUPPORTED: &str = "unsupported_delivery";
}

pub mod failure_kind {
    pub const CONFIGURATION: &str = "configuration";
    pub const REMOTE: &str = "remote";
    pub const TRANSPORT: &str = "transport";
}

pub mod relay_status {
    pub const CONTROL_ACQUIRED: &str = "control_acquired";
    pub const READ_ONLY: &str = "read_only";
    pub const CONTROL_LOST: &str = "control_lost";
    pub const HOST_STARTED: &str = "host_started";
    pub const SESSION_RESUMED: &str = "session_resumed";
    pub const SESSION_ENDING: &str = "session_ending";
    pub const SESSION_EXITED: &str = "session_exited";
}

pub const REQUEST_KIND: &str = "request";
pub const RESPONSE_KIND: &str = "response";
pub const EVENT_KIND: &str = "event";
pub const HOST_SERVER_NAME: &str = "yolo-host";

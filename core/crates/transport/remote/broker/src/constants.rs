use std::time::Duration;
pub const RUNTIME_SOCKET_FILE: &str = "runtime.sock";
pub const MANAGEMENT_SOCKET_FILE: &str = "management.sock";
pub const CONFIG_GENERATIONS_DIRECTORY: &str = "config-generations";
pub const GENERATION_CONFIG_FILE: &str = "config.json";
pub const GENERATION_KNOWN_HOSTS_FILE: &str = "known_hosts";
pub const GENERATION_ARTIFACT_FILE: &str = "runtime-artifact";
pub const GENERATION_IDENTITY_PREFIX: &str = "identity-";
pub const PENDING_GENERATION_PREFIX: &str = "pending-";
pub const PENDING_GENERATION_PAYLOAD_SUFFIX: &str = "-payload";
pub const MAX_CONCURRENT_GENERATION_STAGES: usize = 16;
pub const BROKER_CONFIG_VERSION: u16 = 1;
pub const BROKER_PROTOCOL_VERSION: u16 = 4;
pub const CONNECTION_BOOTSTRAP_CAPABILITY: &str = "connection-bootstrap";
pub const WORKING_DIRECTORY_CAPABILITY: &str = "working-directory";
pub const REMOTE_LOGIN_DIRECTORY: &str = ".";
pub const PRIVATE_DIRECTORY_MODE: u32 = 0o700;
pub const PRIVATE_FILE_MODE: u32 = 0o600;
pub const IMMUTABLE_DIRECTORY_MODE: u32 = 0o500;
pub const IMMUTABLE_FILE_MODE: u32 = 0o400;
pub const READ_ONLY_FILE_MODE_MASK: u32 = 0o022;
pub const PRIVATE_FILE_MODE_MASK: u32 = 0o077;
pub const SOCKET_PERMISSION_MODE_MASK: u32 = 0o7777;
pub const BROKER_COORDINATION_LOCK_MAX_BYTES: u64 = 128;
pub const PERMISSION_BITS_MASK: u32 = 0o777;
pub const EXPECTED_SECURE_FILE_LINKS: u64 = 1;
pub const UNIX_SOCKET_PATH_MAX_BYTES: usize = 104;
pub const MAX_HOST_ID_BYTES: usize = 128;
pub const MAX_IDENTITY_FILES: usize = 32;
pub const MAX_MANAGEMENT_CLIENTS: usize = 256;
pub const MAX_RUNTIME_ATTACHMENTS: usize = 512;
pub const RUNTIME_CONTROL_CLIENT_RESERVE: usize = 32;
pub const MAX_RUNTIME_CLIENTS: usize = MAX_RUNTIME_ATTACHMENTS + RUNTIME_CONTROL_CLIENT_RESERVE;
pub const ACCEPT_FAILURE_INITIAL_BACKOFF: Duration = Duration::from_millis(100);
pub const ACCEPT_FAILURE_MAX_BACKOFF: Duration = Duration::from_secs(2);
pub const MAX_CONFIG_FILE_BYTES: usize = 1_048_576;
pub const MAX_KNOWN_HOSTS_BYTES: usize = 8_388_608;
pub const MAX_IDENTITY_FILE_BYTES: usize = 16_777_216;
pub const MAX_ARTIFACT_FILE_BYTES: usize = yolo_application_support::constants::MAX_RUNTIME_BINARY_BYTES;
pub const INITIAL_RECONNECT_DELAY: Duration = Duration::from_millis(100);
pub const MAX_RECONNECT_DELAY: Duration = Duration::from_secs(5);
pub const BROKER_READY_POLL_INTERVAL: Duration = Duration::from_millis(25);
pub(super) const BROKER_ADOPTION_PENDING_MESSAGE: &str = "broker launch is awaiting adoption";
pub const BROKER_SPAWN_RETRY_INTERVAL: Duration = Duration::from_millis(100);
pub const BROKER_SOCKET_PROBE_TIMEOUT: Duration = Duration::from_millis(250);
pub const BROKER_NAMESPACE_POLL_INTERVAL: Duration = Duration::from_millis(25);
pub const LOCAL_HANDSHAKE_TIMEOUT: Duration = Duration::from_secs(5);
pub const BROKER_SOCKET_PROBE_TIMEOUT_CONTEXT: &str = "broker socket probe timed out";
pub const BROKER_SOCKET_PROBE_ERROR_CONTEXT: &str = "broker socket probe was indeterminate";
pub const BROKER_SOCKET_CHANGED_CONTEXT: &str = "broker socket path changed";
pub const BROKER_SOCKET_TYPE_ERROR: &str = "broker IPC path is not a Unix socket";
pub const BROKER_SOCKET_OWNER_ERROR: &str = "broker socket owner is unsafe";
pub const BROKER_SOCKET_PERMISSION_ERROR: &str = "broker socket permissions are unsafe";
pub const BROKER_SOCKET_STAGING_PREFIX: &str = ".broker-socket-staging-";
pub const BROKER_SOCKET_QUARANTINE_PREFIX: &str = ".broker-socket-quarantine-";
pub const BROKER_SOCKET_QUARANTINE_ATTEMPTS: usize = 4;
pub const BROKER_SOCKET_STAGING_ATTEMPTS: usize = 4;
pub const BROKER_SOCKET_PARENT_ERROR: &str = "broker socket has no parent directory";
pub const BROKER_NAMESPACE_LOST_CONTEXT: &str = "broker namespace identity was lost";
pub const BROKER_STATE_DIRECTORY_LABEL: &str = "broker state directory";
#[cfg(test)]
pub(crate) const TEST_STATE_DIRECTORY: &str = "state";
pub const BROKER_STATE_ANCHOR_INVALID: &str = "invalid broker state anchor";
pub const BROKER_STATE_ANCHOR_MISMATCH: &str = "broker state anchor differs from expected anchor";
pub const BROKER_STATE_DEVICE_INVALID: &str = "broker state device identifier is invalid";
pub const BROKER_SOCKET_DEVICE_INVALID: &str = "broker socket device identifier is invalid";
pub const BROKER_SOCKET_NAME_INVALID: &str = "broker socket name contains a null byte";
pub const BROKER_SOCKET_CHANGED: &str = "broker socket changed";
pub const BROKER_SOCKET_PUBLISH_CHANGED: &str = "published broker socket identity changed";
pub const BROKER_SOCKET_PUBLISH_TARGET_EXISTS: &str =
    "broker socket target appeared during publish";
pub const BROKER_SOCKET_PUBLISH_RESTORE_FAILED: &str = "restore broker socket after failed publish";
pub const BROKER_SOCKET_INSPECT_RETAINED_CONTEXT: &str = "inspect retained broker socket";
pub const BROKER_SOCKET_QUARANTINE_NAME_CONTEXT: &str = "encode broker socket quarantine name";
pub const BROKER_SOCKET_QUARANTINE_EXHAUSTED: &str = "broker socket quarantine capacity exhausted";
pub const BROKER_SOCKET_QUARANTINE_RESTORE_FAILED: &str = "restore broker socket after quarantine";
pub const BROKER_SOCKET_STAGING_EXHAUSTED: &str = "broker socket staging capacity exhausted";
pub const BROKER_SOCKET_STAGING_MISSING: &str = "staged broker socket disappeared";
pub const BROKER_SOCKET_STAGING_LISTENER_MISSING: &str = "staged listener missing";
pub const BROKER_SOCKET_STAGING_NAME_CONTEXT: &str = "encode staged broker socket name";
pub const BROKER_SOCKET_STAGING_BIND_CONTEXT: &str = "bind staged broker socket";
pub const BROKER_SOCKET_STAGING_NONBLOCKING_CONTEXT: &str = "make staged broker socket nonblocking";
pub const BROKER_SOCKET_STAGING_REGISTER_CONTEXT: &str = "register staged broker socket";
pub const BROKER_SOCKET_RETIRE_INSPECT_CONTEXT: &str = "inspect retiring broker socket";
pub const BROKER_RECONCILIATION_LOCK_BUSY: &str = "reconciliation lock unexpectedly busy";
#[cfg(test)]
pub const BROKER_COORDINATION_LOCK_NAME_ERROR: &str = "broker coordination lock has no file name";
pub const BROKER_MANAGEMENT_ENDPOINT_MISSING: &str =
    "captured broker management endpoint is missing";
pub const BROKER_PEER_OWNER_ERROR: &str = "broker peer owner is unsafe";
pub const BROKER_RECONCILE_LOCK_TIMEOUT: &str = "timed out waiting for broker reconciliation lock";
/// Management work is canceled after listeners have closed so handoff cannot stall.
pub const MANAGEMENT_DRAIN_TIMEOUT: Duration = Duration::from_secs(2);
/// A retiring broker must not keep requests that have not opened a remote channel.
pub const RUNTIME_SETUP_DRAIN_TIMEOUT: Duration = Duration::from_secs(2);
/// External termination is bounded even when a runtime client remains connected.
pub const EXTERNAL_RUNTIME_DRAIN_TIMEOUT: Duration = Duration::from_secs(2);
pub const TASK_ABORT_SETTLE_TIMEOUT: Duration = Duration::from_secs(1);
pub const REMOTE_CLEANUP_DRAIN_TIMEOUT: Duration =
    crate::remote::cleanup_settlement_timeout(crate::remote::SSH_OPERATION_TIMEOUT);
/// Runtime sessions may idle indefinitely; only a blocked sink write has this deadline.
pub const RUNTIME_FORWARD_WRITE_TIMEOUT: Duration = Duration::from_secs(5);
pub const RUNTIME_REMOTE_SHUTDOWN_TIMEOUT: Duration = Duration::from_secs(1);
pub const RUNTIME_LOCAL_SHUTDOWN_TIMEOUT: Duration = Duration::from_secs(1);
/// After either input half closes, allow the opposite direction to flush its final bytes.
pub const RUNTIME_TAIL_DRAIN_TIMEOUT: Duration = Duration::from_secs(5);
pub const RUNTIME_FORWARD_BUFFER_BYTES: usize = 16_384;
pub const MAX_RUNTIME_PENDING_INPUT_BYTES: usize = 1_048_576;
pub const DEFAULT_COMMAND_TIMEOUT_MILLIS: u64 = 30_000;
pub const DEFAULT_COMMAND_OUTPUT_BYTES: usize = 1_048_576;
pub const DEFAULT_FILE_READ_BYTES: u64 = 1_048_576;
pub const BROKER_ERROR_CODE: &str = "broker_error";
pub const INVALID_REQUEST_ERROR_CODE: &str = "invalid_request";
pub const CONNECTION_LOST_ERROR_CODE: &str = "connection_lost";
pub const CONFIG_LABEL: &str = "broker config";
pub const KNOWN_HOSTS_LABEL: &str = "known_hosts";
pub const IDENTITY_LABEL: &str = "identity";
pub const ARTIFACT_LABEL: &str = "runtime artifact";
pub const AGENT_SOCKET_LABEL: &str = "SSH agent socket";
pub const RUNTIME_LISTENER_LABEL: &str = "runtime";
pub const MANAGEMENT_LISTENER_LABEL: &str = "management";
pub const BROKER_STATE_NAMESPACE_LABEL: &str = "broker state";
pub const BROKER_RUNTIME_PARENT_NAMESPACE_LABEL: &str = "broker runtime socket parent";
pub const BROKER_MANAGEMENT_PARENT_NAMESPACE_LABEL: &str = "broker management socket parent";
pub const SSH_AUTH_SOCK_ENVIRONMENT: &str = "SSH_AUTH_SOCK";
#[cfg(test)]
#[path = "constants/broker_config_constants.rs"]
mod broker_config_constants;
#[cfg(test)]
pub use broker_config_constants::*;
pub const LOCAL_PROTOCOL_TIMEOUT_MESSAGE: &str = "local broker protocol deadline exceeded";
pub const RUNTIME_LOCAL_WRITE_TIMEOUT_MESSAGE: &str =
    "runtime local-to-remote write deadline exceeded";
pub const RUNTIME_REMOTE_WRITE_TIMEOUT_MESSAGE: &str =
    "runtime remote-to-local write deadline exceeded";
pub const RUNTIME_REMOTE_SHUTDOWN_TIMEOUT_MESSAGE: &str =
    "runtime remote write-half shutdown deadline exceeded";
pub const RUNTIME_LOCAL_SHUTDOWN_TIMEOUT_MESSAGE: &str =
    "runtime local write-half shutdown deadline exceeded";
pub const RUNTIME_REMOTE_TAIL_TIMEOUT_MESSAGE: &str =
    "runtime remote-to-local tail drain deadline exceeded";
pub const RUNTIME_LOCAL_TAIL_TIMEOUT_MESSAGE: &str =
    "runtime local-to-remote tail drain deadline exceeded";
pub const RUNTIME_PENDING_INPUT_LIMIT_MESSAGE: &str =
    "runtime input exceeded the pending-handshake buffer limit";
pub const RUNTIME_ATTACHMENT_LIMIT_MESSAGE: &str = "runtime attachment limit reached";
pub const SSH_IDENTITY_COUNT_LIMIT_MESSAGE: &str = "too many SSH identities";
pub const BROKER_CONFIG_REVISION_MISMATCH_MESSAGE: &str =
    "broker config revision does not match expected revision";
pub const BROKER_PROTOCOL_UNSUPPORTED_MESSAGE: &str = "unsupported broker protocol version";
pub const BROKER_SHUTDOWN_REVISION_CHANGED_MESSAGE: &str = "broker configuration revision changed";
pub const BROKER_SHUTDOWN_INSTANCE_CHANGED_MESSAGE: &str = "broker instance changed";
pub const CONNECTION_GENERATION_EXHAUSTED_MESSAGE: &str = "broker connection generation exhausted";
pub const REMOTE_CLEANUP_FAIL_CLOSED_MESSAGE: &str =
    "broker remote cleanup is indeterminate; retaining broker custody";

pub(super) const ATOMIC_FILE_PREFIX: &str = ".yolo-state-";
pub(super) const ATOMIC_FILE_SUFFIX: &str = ".tmp";
pub(super) const CATALOG_FILE_NAME: &str = "catalog.sqlite";
pub(super) const EVENTS_DIRECTORY: &str = "events";
pub(super) const EXIT_RECORD_FILE_NAME: &str = "exit.json";
pub(super) const HOST_SOCKET_FILE_NAME: &str = "host.sock";
pub(super) const CLAUDE_HOOK_SOCKET_PREFIX: &str = "c-";
pub(super) const CODEX_HOOK_SOCKET_PREFIX: &str = "x-";
pub(super) const QWEN_HOOK_SOCKET_PREFIX: &str = "q-";
pub(super) const PROVIDER_HOOK_SOCKET_PREFIX: &str = "p-";
pub(super) const CUSTODY_GUARDIAN_SOCKET_PREFIX: &str = "g-";
pub(super) const JOURNAL_SEGMENT_FILE_NAME: &str = "journal-00000000.bin";
pub(super) const LOG_DIRECTORY: &str = "logs";
pub(super) const MANIFEST_FILE_NAME: &str = "manifest.json";
pub(super) const RUNTIME_DIRECTORY: &str = "runtime";
pub(super) const RUNTIME_RECORD_FILE_NAME: &str = "runtime.json";
pub(super) const REBOOT_MIGRATION_INTENT_FILE_NAME: &str = "reboot-migration.json";
pub(super) const REBOOT_PREPARATION_AUTHORITY_FILE_NAME: &str = "reboot-preparation.json";
pub(super) const PTY_CUSTODY_FILE_NAME: &str = "pty-custody.json";
pub(crate) const SESSION_LIFECYCLE_LOCK_FILE_NAME: &str = ".lifecycle.lock";
pub(crate) const SESSION_LIFECYCLE_LOCK_MAX_BYTES: u64 = 0;
pub(super) const RESUME_TRANSACTION_FILE_NAME: &str = "resume-transaction.json";
pub(super) const RESUME_BINDING_CLEAR_FILE_NAME: &str = "resume-binding-clear.json";
pub(super) const SESSIONS_DIRECTORY: &str = "sessions";
pub(super) const SNAPSHOT_FILE_NAME: &str = "snapshot.bin";
pub(super) const SOCKET_FILE_EXTENSION: &str = ".sock";
pub(super) const SOCKET_BINDING_SCHEMA_VERSION: u32 = 1;
pub(super) const SOCKET_NAMESPACE_FILE_UNLINK_FLAGS: i32 = 0;
pub(super) const SOCKET_DIGEST_DOMAIN: &str = "yolo.socket-namespace.v1";
pub(super) const SOCKET_DIGEST_SEPARATOR: u8 = 0;
pub(super) const SOCKET_FULL_HASH_BYTES: usize = 32;
pub(super) const SOCKET_FULL_HASH_HEX_LENGTH: usize =
    SOCKET_FULL_HASH_BYTES * crate::runtime_constants::HEX_DIGITS_PER_BYTE;
pub(super) const SOCKET_HASH_BYTES: usize = 16;
pub(super) const SOCKET_HASH_HEX_LENGTH: usize =
    SOCKET_HASH_BYTES * crate::runtime_constants::HEX_DIGITS_PER_BYTE;
pub(super) const SOCKET_LIFECYCLE_LOCK_RETRY_MILLIS: u64 = 5;
pub(super) const SOCKET_LIFECYCLE_LOCK_TIMEOUT_SECS: u64 = 8;
pub(super) const SOCKET_LIFECYCLE_LOCK_TIMEOUT: std::time::Duration =
    std::time::Duration::from_secs(SOCKET_LIFECYCLE_LOCK_TIMEOUT_SECS);
#[cfg(target_os = "macos")]
pub(super) const ISOLATED_TEST_SOCKET_PARENT: &str = "/private/tmp";
#[cfg(all(unix, not(target_os = "macos")))]
pub(super) const ISOLATED_TEST_SOCKET_PARENT: &str = "/tmp";
pub(super) const ISOLATED_TEST_SOCKET_PREFIX: &str = "yn-";
pub(super) const ISOLATED_TEST_SOCKET_TOKEN_HEX_LENGTH: usize = 8;
pub(super) const ISOLATED_TEST_SOCKET_CREATION_ATTEMPTS: usize = 8;
pub(super) const ISOLATED_TEST_SOCKET_ROOT_MAX_BYTES: usize = 24;
pub(super) const ISOLATED_TEST_SOCKET_RETIRED_PREFIX: &str = "yr-";
pub(super) const SOCKET_NAMESPACE_SWEEP_ENTRY_BUDGET: usize = 16;
pub(super) const SOCKET_NAMESPACE_HOST_ENDPOINT_COUNT: usize = 1;
const SOCKET_NAMESPACE_HUB_ENDPOINT_COUNT: usize = 1;
const SOCKET_NAMESPACE_CONNECTOR_ENDPOINT_COUNT: usize = 1;
const SOCKET_NAMESPACE_PUBLIC_CLIENT_ENDPOINT_COUNT: usize = 1;
const SOCKET_NAMESPACE_WORKER_ENDPOINT_COUNT_PER_SESSION: usize = 1;
const SOCKET_NAMESPACE_CLAUDE_ENDPOINT_COUNT_PER_SESSION: usize = 1;
const SOCKET_NAMESPACE_CODEX_ENDPOINT_COUNT_PER_SESSION: usize = 1;
const SOCKET_NAMESPACE_GUARDIAN_ENDPOINT_COUNT_PER_SESSION: usize = 1;
pub(super) const SOCKET_NAMESPACE_SESSION_ENDPOINT_COUNT: usize =
    SOCKET_NAMESPACE_WORKER_ENDPOINT_COUNT_PER_SESSION
        + SOCKET_NAMESPACE_CLAUDE_ENDPOINT_COUNT_PER_SESSION
        + SOCKET_NAMESPACE_CODEX_ENDPOINT_COUNT_PER_SESSION
        + SOCKET_NAMESPACE_GUARDIAN_ENDPOINT_COUNT_PER_SESSION;
pub(super) const SOCKET_NAMESPACE_PROOF_ENDPOINT_LIMIT: usize =
    crate::runtime_constants::MAX_MANAGED_SESSIONS * SOCKET_NAMESPACE_SESSION_ENDPOINT_COUNT
        + SOCKET_NAMESPACE_HOST_ENDPOINT_COUNT
        + SOCKET_NAMESPACE_HUB_ENDPOINT_COUNT
        + SOCKET_NAMESPACE_CONNECTOR_ENDPOINT_COUNT
        + SOCKET_NAMESPACE_PUBLIC_CLIENT_ENDPOINT_COUNT;
pub(super) const SOCKET_STAGE_CLEANUP_BUDGET: usize = 1;
pub(super) const SOCKET_STAGE_SWEEP_ENTRY_BUDGET: usize = 64;
pub(super) const SOCKET_STAGE_TARGET_LIMIT: usize = SOCKET_NAMESPACE_PROOF_ENDPOINT_LIMIT;
pub(super) const SOCKET_STAGE_RETIRED_CHUNK_LIMIT: usize =
    SOCKET_STAGE_TARGET_LIMIT + MAX_PRIVATE_DIRECTORY_ENTRIES;
pub(super) const SOCKET_STAGE_LEASE_MAX_BYTES: u64 = 256;
pub(super) const SOCKET_STAGE_LIFECYCLE_MAX_BYTES: u64 = 0;
pub(super) const SOCKET_STAGE_LEASE_RETRY_MILLIS: u64 = 5;
pub(super) const SOCKET_STAGE_LEASE_TIMEOUT_SECS: u64 = 1;
pub(super) const SOCKET_STAGE_LEASE_TIMEOUT: std::time::Duration =
    std::time::Duration::from_secs(SOCKET_STAGE_LEASE_TIMEOUT_SECS);
pub(super) const SOCKET_ENDPOINT_LINK_COUNT: u64 = EXPECTED_FILE_LINK_COUNT;
pub(super) const SOCKET_PROBE_LINK_COUNT: u64 =
    SOCKET_ENDPOINT_LINK_COUNT + EXPECTED_FILE_LINK_COUNT;
pub(super) const SOCKET_NAMESPACE_CLEANUP_FAILED_LOG: &str =
    "failed to clean an unused socket namespace";
pub const SOCKET_ROOT_ENVIRONMENT: &str = "YOLO_SOCKET_ROOT";
#[cfg(target_os = "macos")]
pub(super) const SOCKET_ROOT_PREFIX: &str = "/private/tmp/y-";
#[cfg(all(unix, not(target_os = "macos")))]
pub(super) const SOCKET_ROOT_PREFIX: &str = "/tmp/y-";

#[cfg(test)]
pub(super) const PROCESS_FILE_LOCK_TEST_HELPER_FILTER: &str = "kernel_lock_subprocess_helper";
#[cfg(test)]
pub(super) const PRIVATE_FILE_LOCK_WAIT_TEST_HELPER_FILTER: &str =
    "private_file_lock_wait_subprocess_helper";
#[cfg(test)]
pub(super) const PRIVATE_FILE_LOCK_FIRST_CREATE_HELPER_FILTER: &str =
    "private_file_lock_first_create_subprocess_helper";
#[cfg(test)]
pub(super) const PROCESS_FILE_LOCK_TEST_NOCAPTURE_ARGUMENT: &str = "--nocapture";
#[cfg(test)]
pub(super) const SOCKET_NAMESPACE_ROOT_LOCK_TEST_HELPER_FILTER: &str =
    "socket_namespace_root_inode_lock_subprocess_helper";
#[cfg(test)]
pub(super) const PROCESS_FILE_LOCK_TEST_SIGNAL_BYTES: &[u8] = b"signal";
#[cfg(test)]
pub(super) const TEST_SOCKET_ROOT_OVERRIDE_ERROR: &str = "test socket root override must be valid";
pub const UNIX_SOCKET_PATH_MAX_BYTES: usize = 100;
pub const HUB_SOCKET_FILE_NAME: &str = "hub.sock";
pub const CONNECTOR_SOCKET_FILE_NAME: &str = "connector.sock";
pub const PUBLIC_CLIENT_SOCKET_FILE_NAME: &str = "public-client.sock";
pub(super) const TERMINAL_DIRECTORY: &str = "terminal";
pub(super) const TERMINATE_TOMBSTONE_FILE_NAME: &str = "terminate.json";
pub(super) const WORKER_LAUNCH_FILE_NAME: &str = "worker-launch.json";
pub(super) const WORKER_STARTUP_CHECKPOINT_FILE_NAME: &str = "worker-startup.json";
pub const WORKER_STARTUP_CHECKPOINT_MAX_BYTES: u64 = 8_192;

#[cfg(unix)]
pub(crate) const PRIVATE_DIRECTORY_MODE: u32 = 0o700;
#[cfg(unix)]
pub const PRIVATE_FILE_MODE: u32 = 0o600;
#[cfg(unix)]
pub const NON_OWNER_PERMISSION_MASK: u32 = 0o077;
pub(super) const EXPECTED_FILE_LINK_COUNT: u64 = 1;
pub(super) const GROUP_OR_OTHER_WRITE_MODE: u32 = 0o022;
pub const MAX_DURABLE_JSON_BYTES: u64 = 1_048_576;
pub const MAX_PRIVATE_DIRECTORY_ENTRIES: usize = 256;
pub(super) const CURRENT_DIRECTORY_ENTRY: &str = ".";
pub(super) const PARENT_DIRECTORY_ENTRY: &str = "..";
pub const PERMISSION_MODE_MASK: u32 = 0o7777;
pub(super) const PRIVATE_FILE_NAME_HASH_BYTES: usize = 16;
pub(crate) const PRIVATE_FILE_NAME_HASH_HEX_LENGTH: usize =
    PRIVATE_FILE_NAME_HASH_BYTES * crate::runtime_constants::HEX_DIGITS_PER_BYTE;
pub(super) const PRIVATE_FILE_QUARANTINE_PREFIX: &str = ".yolo-private-quarantine-";
pub(crate) const PRIVATE_FILE_RETIRED_PREFIX: &str = ".yolo-private-retired-";
pub(crate) const PRIVATE_FILE_RETAINED_ISOLATION_PREFIX: &str = ".yolo-private-retained-isolation-";
pub(crate) const PRIVATE_FILE_UNLINK_ISOLATION_PREFIX: &str = ".yolo-private-unlink-isolation-";
pub(crate) const PRIVATE_FILE_STAGING_PREFIX: &str = ".yolo-private-staging-";
pub(super) const PRIVATE_FILE_TEMPORARY_ATTEMPTS: usize = 200;
pub(super) const PRIVATE_FILE_TEMPORARY_RETRY_MILLIS: u64 = 1;
pub(super) const PRIVATE_FILE_TEMPORARY_RETRY_ERROR: &str =
    "private staging identity kept changing";
pub(super) const ROOT_USER_ID: u32 = 0;
pub(super) const SIZE_LIMIT_SENTINEL_BYTES: u64 = 1;
pub(super) const SPECIAL_PERMISSION_MODE: u32 = 0o7000;
pub(super) const STICKY_MODE: u32 = 0o1000;

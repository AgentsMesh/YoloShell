pub const ENTRY_DIRECTORY: &str = "offline-entry";
pub const RELAY_DIRECTORY: &str = "offline-relay";
pub const ORIGIN_DIRECTORY: &str = "offline-origin";
pub const ENTRY_AGENT: &str = "offline-entry-agent";
pub const RELAY_AGENT: &str = "offline-relay-agent";
pub const ORIGIN_AGENT: &str = "offline-origin-agent";
pub const EXPECTED_AGENTS: usize = 3;
pub const EXPECTED_REMOTE_AGENTS: usize = 2;
pub const REJECTION_DETAIL: &str = "offline observation incorrectly authorized a remote operation";
pub const CACHE_SCHEMA: u16 = 2;
pub const CACHE_RECORD_EXTENSION: &str = "json";
pub const CACHE_ROOT_LABEL: &str = "offline-cache-fixture-root";
pub const CACHE_DIRECTORY_LABEL: &str = "offline-cache-fixture-observations";
pub const CACHE_PROBE_THREAD: &str = "offline-cache-evidence";
pub const CACHE_PROBE_TIMEOUT: &str =
    "expected remote observations were not published to the cache";

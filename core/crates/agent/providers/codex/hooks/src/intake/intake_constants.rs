pub(super) const MAX_BYTES: u64 = crate::paths::MAX_DURABLE_JSON_BYTES;
pub(super) const MAX_ARTIFACTS_PER_OCCURRENCE: usize = 2;
pub(super) const MAX_PENDING_OCCURRENCES: usize = crate::agent::hooks::MAX_REPLAY_WINDOW;
pub(super) const MAX_DIRECTORY_ENTRIES: usize =
    MAX_PENDING_OCCURRENCES.saturating_mul(MAX_ARTIFACTS_PER_OCCURRENCE);
pub(super) const RECORD_SCHEMA_VERSION: u16 = 1;
pub(super) const QUARANTINE_SCHEMA_VERSION: u16 = 2;
pub(super) const DIRECTORY_NAME: &str = "intake";
pub(super) const EVENT_SUFFIX: &str = ".event.json";
pub(super) const QUARANTINE_SUFFIX: &str = ".quarantine.json";
pub(super) const NAME_SEPARATOR: char = '-';

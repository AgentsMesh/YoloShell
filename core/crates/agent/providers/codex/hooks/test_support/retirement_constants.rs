pub(super) const ACQUIRED_ENV: &str = "YOLO_CODEX_RETIREMENT_RACE_ACQUIRED";
pub(super) const ADMISSION_FILE_IDENTITY_FIELD: &str = "admission_file_identity";
pub(super) const BOOT_FIELD: &str = "boot";
pub(super) const CHANNEL_CAPACITY: usize = 0;
pub(super) const CONTENDED_ENV: &str = "YOLO_CODEX_RETIREMENT_RACE_CONTENDED";
pub(super) const CONTEXT_ENV: &str = "YOLO_CODEX_RETIREMENT_RACE_CONTEXT";
pub(super) const DIGEST_BYTE_INDEX: usize = 0;
pub(super) const DIGEST_BYTE_OFFSET: u8 = 1;
pub(super) const DISPLACED_INVOCATION_DIRECTORY: &str = "displaced-invocation";
pub(super) const EXPECTED_ENV: &str = "YOLO_CODEX_RETIREMENT_RACE_EXPECTED";
pub(super) const EXPECTED_FAILURE: &str = "failure";
pub(super) const EXPECTED_SUCCESS: &str = "success";
pub(super) const HELPER_FILTER: &str = "codex_retirement_writer_fence_subprocess_helper";
pub(super) const HELPER_MODE: &str = "writer-fence-race";
pub(super) const HELPER_TIMEOUT_SECONDS: u64 = 10;
pub(super) const HELPER_TIMEOUT: std::time::Duration =
    std::time::Duration::from_secs(HELPER_TIMEOUT_SECONDS);
pub(super) const INODE_FIELD: &str = "inode";
pub(super) const KNOWN_FIELD: &str = "known";
pub(super) const MISSING_ADMISSION: &str = "Codex process-race admission is missing";
pub(super) const MISSING_GONE_PROOF: &str = "Codex process-race gone proof is unavailable";
pub(super) const MODE_ENV: &str = "YOLO_CODEX_RETIREMENT_RACE_MODE";
pub(super) const NOCAPTURE_ARGUMENT: &str = "--nocapture";
pub(super) const POLL_INTERVAL_MILLIS: u64 = 5;
pub(super) const POLL_INTERVAL: std::time::Duration =
    std::time::Duration::from_millis(POLL_INTERVAL_MILLIS);
pub(super) const PRODUCER_ACQUIRED_FILE: &str = "producer-acquired";
pub(super) const PRODUCER_CONTENDED_FILE: &str = "producer-contended";
pub(super) const PRODUCER_RELEASE_FILE: &str = "producer-release";
pub(super) const PRODUCER_ROLE: &str = "producer";
pub(super) const PRODUCER_WIN_OCCURRENCE_COUNT: usize = 1;
pub(super) const RELEASE_ENV: &str = "YOLO_CODEX_RETIREMENT_RACE_RELEASE";
pub(super) const ROLE_ENV: &str = "YOLO_CODEX_RETIREMENT_RACE_ROLE";
pub(super) const SCHEMA_VERSION_FIELD: &str = "schema_version";
pub(super) const SEAL_FIELD: &str = "seal";
pub(super) const SEAL_FILE_IDENTITY_FIELD: &str = "seal_file_identity";
pub(super) const SEAL_SHA256_FIELD: &str = "seal_sha256";
pub(super) const SEALER_ACQUIRED_FILE: &str = "sealer-acquired";
pub(super) const SEALER_CONTENDED_FILE: &str = "sealer-contended";
pub(super) const SEALER_RELEASE_FILE: &str = "sealer-release";
pub(super) const SEALER_ROLE: &str = "sealer";
pub(super) const SIGNAL_BYTES: &[u8] = b"signal";
pub(super) const SNAPSHOT_FIELD: &str = "snapshot";
pub(super) const SNAPSHOT_LINEAGE_FIELD: &str = "snapshot_lineage";
pub(super) const SNAPSHOT_SHA256_FIELD: &str = "snapshot_sha256";
pub(super) const UNEXPECTED_ROLE: &str = "unexpected process-race role";
pub(super) const UNKNOWN_FIELD: &str = "future_retirement_field";
pub(super) const UNKNOWN_VALUE: bool = true;
pub(super) const UNSIGNED_OFFSET: u64 = 1;

pub(super) const UNKNOWN_FIELD_LOCATIONS: &[&[&str]] = &[
    &[],
    &[SEAL_FIELD],
    &[SEAL_FIELD, ADMISSION_FILE_IDENTITY_FIELD],
    &[SEAL_FIELD, BOOT_FIELD],
    &[SEAL_FILE_IDENTITY_FIELD],
    &[SNAPSHOT_LINEAGE_FIELD],
    &[SNAPSHOT_LINEAGE_FIELD, SNAPSHOT_FIELD],
];

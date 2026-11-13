pub(super) const RECORD_SCHEMA_VERSION: u16 = 2;
pub(super) const SEAL_SCHEMA_VERSION: u16 = 1;

pub(super) const ADMISSION_DIGEST_DOMAIN: &str = "yolo.codex.retirement.admission.v1";
pub(super) const SEAL_DIGEST_DOMAIN: &str = "yolo.codex.retirement.seal.v1";
pub(super) const SNAPSHOT_DIGEST_DOMAIN: &str = "yolo.codex.retirement.snapshot.v1";

pub(super) const FILE_NAME: &str = "retirement.json";
pub(super) const SEAL_FILE_NAME: &str = "retirement-seal.json";
pub(super) const WRITER_LOCK_FILE_NAME: &str = "intake-writer.lock";

pub(super) const CORRUPT: &str = "Codex hook retirement authority is corrupt";
pub(super) const INTAKE_RETIRED: &str = "Codex hook invocation no longer accepts durable intake";
pub(super) const STORAGE: &str = "Codex hook retirement storage failed";

#[cfg(test)]
pub(crate) use yolo_distribution_publication_protocol::FIRST_REVISION;
pub use yolo_distribution_publication_protocol::{
    CURRENT_FILE_NAME, EXECUTABLE_MODE, MANIFEST_MODE, SELECTION_SCHEMA_VERSION, STAGE_PREFIX,
};
pub(crate) const LOCK_FILE_NAME: &str = ".publication.lock";
pub(crate) const LOCK_BYTES: u64 = 4096;
pub(crate) const MAX_MANIFEST_BYTES: u64 = 64 * 1024;
pub(crate) const ROOT_NAMESPACE: &str = "runtime bundle installation";
pub(crate) const RELEASES_NAMESPACE: &str = "runtime bundle releases";
pub(crate) const PERMISSION_MASK: u32 = 0o7777;
pub(crate) const SINGLE_FILE_LINK: u64 = 1;
pub(crate) const HASH_CHUNK_BYTES: usize = 64 * 1024;
pub(crate) const HEADER_PROBE_BYTES: usize = 64;
pub(crate) const BUSY: &str = "runtime publication is busy";
pub(crate) const CONFLICT: &str = "runtime selection changed before publication";
pub(crate) const OPERATION_REUSE: &str = "publication operation identifies another build";
pub(crate) const INVALID_OPERATION: &str = "publication and stage UUIDs must be non-nil";
pub(crate) const RELEASE_MISMATCH: &str = "release manifest differs from requested bundle";
pub(crate) const RELEASES_MISSING: &str = "selected runtime has no release directory";
pub(crate) const IDENTITY_CHANGED: &str = "runtime release identity changed";
pub(crate) const INVALID_FILE: &str = "runtime release file metadata is unsafe";
pub(crate) const INVENTORY_MISMATCH: &str =
    "runtime release contains missing or unexpected entries";
pub(crate) const FILE_INTEGRITY: &str = "runtime release file failed integrity verification";
pub(crate) const TARGET_MISMATCH: &str = "runtime release binary target differs";
pub(crate) const PUBLICATION_UNCERTAIN: &str =
    "publication failed; inspect selection and release before retrying";

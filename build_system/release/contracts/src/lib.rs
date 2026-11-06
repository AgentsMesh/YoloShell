mod container;
mod delivery;
mod linux;
mod process;
mod snapshot;

pub use container::*;
pub use delivery::*;
pub use linux::*;
pub use process::*;
pub use snapshot::*;

pub const SCHEMA_VERSION: u16 = 9;
pub const PRODUCT_NAME: &str = "Yolo";
pub const ARCHIVE_NAME: &str = "Yolo.zip";
pub const CHECKSUM_NAME: &str = "Yolo.zip.sha256";
pub const PROVENANCE_NAME: &str = "Yolo.provenance.json";
pub const OUTPUT_FILE_NAMES: &[&str] = &[ARCHIVE_NAME, CHECKSUM_NAME, PROVENANCE_NAME];
pub const CANDIDATE_UNACCEPTED_STATUS: &str = "candidate_unaccepted";
pub const INTEGRITY_ONLY_SCOPE: &str = "artifact_integrity_only";
pub const CHECKSUM_SEPARATOR: &str = "  ";
pub const CHECKSUM_LINE_ENDING: &str = "\n";
pub const PRIVATE_DIRECTORY_MODE: u32 = 0o700;
pub const SEALED_DIRECTORY_MODE: u32 = 0o500;
pub const PRIVATE_FILE_MODE: u32 = 0o600;
pub const READ_ONLY_FILE_MODE: u32 = 0o400;
pub const PRIVATE_EXECUTABLE_MODE: u32 = 0o500;
pub const EXECUTABLE_PERMISSION_MASK: u32 = 0o111;
pub const UNSAFE_DIRECTORY_WRITE_MASK: u32 = 0o022;
pub const NO_FILE_FLAGS: u32 = 0;
pub const PERMISSION_BITS_MASK: u32 = 0o777;
pub const EXPECTED_FILE_LINKS: u64 = 1;

pub const RANDOM_DEVICE: &str = "/dev/urandom";
pub const OUTPUT_TEMPORARY_PREFIX: &str = ".yolo-candidate-";
pub const RELEASE_EVIDENCE_CLI_NAME: &str = "yolo-release-evidence";
pub const CANDIDATE_OPERATION: &str = "candidate";
pub const VERIFY_OPERATION: &str = "verify";
pub const VERIFY_ACCEPTANCE_OPERATION: &str = "verify-acceptance";
pub const WORKSPACE_ARGUMENT: &str = "workspace";
pub const OUTPUT_DIRECTORY_ARGUMENT: &str = "output-directory";

pub const ARTIFACT_CHANGED_ERROR: &str = "pinned candidate artifact changed";
pub const PUBLISHED_ARTIFACT_CHANGED_ERROR: &str = "published candidate artifact changed";
pub const OUTPUT_ENTRIES_ERROR: &str = "candidate output entries mismatch";
pub const COMMITTED_ACKNOWLEDGEMENT_ERROR: &str =
    "candidate committed but publication acknowledgement was lost";
pub const COMMITTED_VALIDATION_ERROR: &str =
    "candidate commit marker is durable but final validation failed or is uncertain";
pub const PUBLICATION_REVERIFY_ERROR: &str = "committed candidate changed after commit marker";
pub const CANDIDATE_EVIDENCE_CHANGED_ERROR: &str = "candidate evidence changed while read";
pub const CLOCK_ERROR: &str = "system clock precedes the Unix epoch";

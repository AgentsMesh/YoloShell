pub use yolo_startup_checkpoint_contract::constants::*;

#[cfg(test)]
pub(crate) const REGULAR_FILE_OBJECT_KIND: &str = "regular_file";

#[cfg(test)]
pub(crate) const UNKNOWN_FIELD: &str = "unknown_private_field";
#[cfg(test)]
pub(crate) const PENDING_PTY_CUSTODY_ROLLOVER_FIELD: &str = "pending_pty_custody_rollover";
#[cfg(test)]
pub(crate) const PTY_CUSTODY_SOURCE_VERSION_FIELD: &str = "source_version";
#[cfg(test)]
pub(crate) const PTY_CUSTODY_SOURCE_SHA256_FIELD: &str = "source_sha256";
#[cfg(test)]
pub(crate) const SANITIZED_FIELDS: [&str; 12] = [
    "boot_id",
    "child",
    "controller_generation",
    "launch_sha256",
    "phase",
    "runtime_epoch",
    "schema_version",
    "stage",
    "status",
    "terminal_id",
    "worker",
    "worker_id",
];
#[cfg(test)]
pub(crate) const SUCCESSOR_FIELD: &str = "successor";
#[cfg(test)]
pub(crate) const PREDECESSOR_RUNTIME_FIELD: &str = "predecessor_runtime_sha256";
#[cfg(test)]
pub(crate) const SANITIZED_FENCE_FIELDS: [&str; 5] = [
    "controller_generation",
    "launch_sha256",
    "runtime_epoch",
    "terminal_id",
    "worker_id",
];

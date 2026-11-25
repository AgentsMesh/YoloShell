pub use yolo_paths::durable_mutation::constants::{
    PARTICIPANT_PATH_INVALID, SESSION_DIRECTORY_MISMATCH,
};

pub(super) const REBOOT_INTENT_SCHEMA_VERSION: u16 = 7;
pub(super) const REBOOT_PREPARATION_SCHEMA_VERSION: u16 = 3;
pub(super) const REBOOT_RECEIPT_SCHEMA_VERSION: u16 = 2;
pub(super) const REBOOT_STAGE_PREFIX: &str = ".reboot-stage-";
pub(super) const REBOOT_RECEIPT_PREPARED_PREFIX: &str = ".reboot-receipt-prepared-";
pub(super) const REBOOT_RECEIPT_INSTALLED_PREFIX: &str = ".reboot-receipt-installed-";
pub(super) const INTENT_INVALID: &str = "durable reboot migration intent is invalid";
pub(super) const INTENT_CHANGED: &str = "durable reboot migration intent changed";
pub(super) const PREPARATION_INVALID: &str = "durable reboot preparation authority is invalid";
pub(super) const PREPARATION_CHANGED: &str = "durable reboot preparation authority changed";
pub(super) const PREPARATION_SOURCE_CHANGED: &str =
    "durable reboot preparation participant source changed";
pub(super) const PREPARATION_ARTIFACT_CHANGED: &str = "durable reboot preparation artifact changed";
pub(super) const PARTICIPANT_CONFLICT: &str = "durable reboot migration participant changed";
pub(super) const PARTICIPANT_RECEIPT_CONFLICT: &str =
    "durable reboot participant install receipt changed";
pub(super) const STARTUP_CHECKPOINT_PARTICIPANT: &str = "startup-checkpoint";
pub(super) const CLAUDE_CUSTODY_PARTICIPANT: &str = "claude-custody";
pub(super) const CLAUDE_GENERATION_PARTICIPANT: &str = "claude-generation";
pub(super) const PTY_CUSTODY_PARTICIPANT: &str = "pty-custody";
pub(super) const RUNTIME_PARTICIPANT: &str = "runtime";
pub(super) const ROLLOVER_SOURCE_PROOF_INVALID: &str =
    "durable reboot PTY custody rollover source proof is invalid";
pub(super) const ROLLOVER_TARGET_INVALID: &str =
    "durable reboot PTY custody rollover target is invalid";
pub(super) const SESSION_CAPACITY_EXCEEDED: &str =
    "durable reboot session inventory exceeds capacity";

#[cfg(test)]
pub(super) mod test_field {
    pub(in crate::durable_reboot) const CLAUDE_CUSTODY: &str = "claude_custody";
    pub(in crate::durable_reboot) const CLAUDE_GENERATION: &str = "claude_generation";
    pub(in crate::durable_reboot) const COMMAND: &str = "command";
    pub(in crate::durable_reboot) const PTY_CUSTODY: &str = "pty_custody";
    pub(in crate::durable_reboot) const STARTUP_CHECKPOINT: &str = "startup_checkpoint";
    pub(in crate::durable_reboot) const TARGET_PAYLOAD: &str = "target_payload";
    pub(in crate::durable_reboot) const TARGET_SHA256: &str = "target_sha256";
    pub(in crate::durable_reboot) const WORKER_PID: &str = "worker_pid";
}

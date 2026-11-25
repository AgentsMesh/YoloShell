use super::RecoveryState;

pub(super) const TERMINAL_MANIFEST_DIGEST_DOMAIN: &[u8] = b"yolo.terminal-manifest-fence.v1";
pub(super) const TERMINAL_MANIFEST_DIGEST_INVALID_ENCODING_DETAIL: &str =
    "terminal manifest digest is not lowercase SHA-256";
pub(super) const TERMINAL_MANIFEST_NON_UNICODE_WORKING_DIRECTORY_DETAIL: &str =
    "terminal manifest working directory is not UTF-8";

const RECOVERY_CREATED: &str = "created";
const RECOVERY_REATTACHED: &str = "reattached";
const RECOVERY_RESUMED: &str = "resumed";
const RECOVERY_RECREATED: &str = "recreated";
const RECOVERY_RESTORING: &str = "restoring";
const RECOVERY_UNCERTAIN: &str = "uncertain";
const RECOVERY_NEEDS_RECREATE: &str = "needs_recreate";

pub(super) const fn recovery_state_name(state: RecoveryState) -> &'static str {
    match state {
        RecoveryState::Created => RECOVERY_CREATED,
        RecoveryState::Reattached => RECOVERY_REATTACHED,
        RecoveryState::Resumed => RECOVERY_RESUMED,
        RecoveryState::Recreated => RECOVERY_RECREATED,
        RecoveryState::Restoring => RECOVERY_RESTORING,
        RecoveryState::Uncertain => RECOVERY_UNCERTAIN,
        RecoveryState::NeedsRecreate => RECOVERY_NEEDS_RECREATE,
    }
}

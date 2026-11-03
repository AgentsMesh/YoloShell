use crate::agent::hooks::HookViolation;
pub(in crate::worker::codex_hook) fn is_authority_lost_error(error: &anyhow::Error) -> bool {
    crate::codex_hooks::is_snapshot_authority_changed(error)
        || error.downcast_ref::<HookViolation>() == Some(&HookViolation::Revoked)
}

pub(super) fn authority_lost_error() -> anyhow::Error {
    anyhow::Error::new(HookViolation::Revoked)
        .context(super::super::constants::CODEX_AUTHORITY_LOST_ERROR)
}

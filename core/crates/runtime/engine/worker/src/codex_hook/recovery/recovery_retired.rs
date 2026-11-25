use crate::codex_hooks::{
    CodexAdmissionAuthority, CodexHookArtifacts, CodexOutbox, CodexRetiredState,
    CodexRetirementWriterFence, CodexSnapshotCursor,
};
use anyhow::{Context, Result};

#[cfg(test)]
#[cfg(any(not(test), feature = "yolo-test-support"))]
#[path = "../../../test_support/unit/codex_hook/recovery/recovery_retired_test_hook.rs"]
mod test_hook;

pub(super) fn verify(
    artifacts: &CodexHookArtifacts,
    writer: CodexRetirementWriterFence,
    admission: CodexAdmissionAuthority,
    retired: CodexRetiredState,
) -> Result<()> {
    let owner = &admission.record().request().identity;
    let outbox = CodexOutbox::for_identity(artifacts.outbox_path(), owner)?;
    let (cursor, snapshot) = CodexSnapshotCursor::open(artifacts.snapshot_path())?;
    let snapshot = snapshot.context(super::constants::CODEX_ROUTE_RECOVERY_ERROR)?;
    let outbox = outbox.prove_terminal(&snapshot, &admission)?;
    let proof = cursor.prove_current()?;
    let intake = writer.prove_terminal_intake(retired.seal(), &admission)?;
    #[cfg(test)]
    test_hook::run();
    writer.verify_retirement_drained(
        retired.retirement(),
        retired.seal(),
        &snapshot,
        &admission,
        &proof,
        &intake,
        &outbox,
    )
}

#[cfg(test)]
pub(super) fn set_before_final_validation_hook(hook: impl FnOnce() + 'static) {
    test_hook::set(hook);
}

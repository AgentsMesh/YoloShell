use crate::codex_hooks::{CodexAdmissionAuthority, CodexHookArtifacts, CodexOutbox};
use anyhow::{Result, ensure};

const NATIVE_FIRST_EVENT: &str = "native Codex registration has inconsistent first event authority";

#[cfg(all(test, feature = "yolo-test-support"))]
#[path = "../../../test_support/unit/codex_hook/native/native_first_event.rs"]
mod test_support;
#[cfg(all(test, feature = "yolo-unit-tests"))]
#[cfg(any(feature = "yolo-integration-tests", feature = "yolo-worker-unit-runtime"))]
#[path = "../../../unit_tests/codex_hook/native/native_first_event_tests.rs"]
mod tests;

pub(super) fn prime(
    paths: &crate::paths::Paths,
    admission: &CodexAdmissionAuthority,
) -> Result<()> {
    admission.record().verify_native_agent()?;
    prime_verified(paths, admission)
}

pub(super) fn prime_sealed(
    paths: &crate::paths::Paths,
    admission: &CodexAdmissionAuthority,
    writer: &crate::codex_hooks::CodexRetirementWriterFence,
    seal: &crate::codex_hooks::CodexRetirementSealArtifact,
) -> Result<()> {
    writer.verify_seal(seal, admission)?;
    prime_verified(paths, admission)?;
    writer.verify_seal(seal, admission)
}

fn prime_verified(paths: &crate::paths::Paths, admission: &CodexAdmissionAuthority) -> Result<()> {
    let first = admission.record().first_event();
    admission.verify()?;
    let owner = &admission.record().request().identity;
    let artifacts = CodexHookArtifacts::new(
        paths,
        owner.terminal_id,
        owner.runtime_epoch,
        owner.invocation_id,
    )?;
    let outbox = CodexOutbox::for_identity(artifacts.outbox_path(), owner)?;
    let state = outbox.load()?;
    if state.next_sequence() == crate::agent::hooks::FIRST_SEQUENCE {
        let mut event = first.clone();
        event.sequence = crate::agent::hooks::EventSequence(crate::agent::hooks::FIRST_SEQUENCE);
        outbox.enqueue_admitted_decision(
            event,
            None,
            |_, _| Ok(crate::codex_hooks::CodexEnqueueDecision::accepted(())),
            |_, _, _| admission.verify(),
        )?;
    } else {
        ensure!(
            state.next_sequence() > crate::agent::hooks::FIRST_SEQUENCE,
            NATIVE_FIRST_EVENT
        );
    }
    admission.verify()
}

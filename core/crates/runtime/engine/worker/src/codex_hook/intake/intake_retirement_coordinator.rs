use super::super::router::{CodexRetirementPermit, CodexRetirementReadiness};
use super::super::runtime::{CodexHookRuntime, CodexRetirementIntakeAuthority};
use super::report::IntakeRouteReport;
use crate::codex_hooks::{
    CodexAdmissionAuthority, CodexRetirementSeal, CodexRetirementSealArtifact,
    CodexRetirementState, CodexRetirementWriterFence,
};
use crate::process::ExactProcessGone;
use anyhow::Result;
use std::sync::Arc;

#[cfg(test)]
#[cfg(any(not(test), feature = "yolo-test-support"))]
#[path = "../../../test_support/unit/codex_hook/intake/intake_retirement_test_fault.rs"]
mod test_fault;
#[cfg(test)]
pub(in crate::worker::codex_hook) use test_fault::RetirementFaultStage;
#[cfg(test)]
pub(in crate::worker::codex_hook) use test_fault::clear_test_faults;

pub(super) fn reconcile(
    readiness: CodexRetirementReadiness,
    gone: Option<ExactProcessGone>,
) -> Result<IntakeRouteReport> {
    match readiness {
        CodexRetirementReadiness::Pending => Ok(pending()),
        CodexRetirementReadiness::Missing => Ok(IntakeRouteReport::RetirementMissing),
        CodexRetirementReadiness::Ready(permit) => retire(permit, gone),
    }
}

fn retire(
    permit: CodexRetirementPermit,
    gone: Option<ExactProcessGone>,
) -> Result<IntakeRouteReport> {
    permit.verify().map_err(anyhow::Error::new)?;
    let runtime = Arc::clone(permit.entry().runtime());
    let admission = permit.entry().admission().authority().clone();
    let writer = admission.acquire_retirement_writer_fence()?;
    match writer.read_state(&admission)? {
        CodexRetirementState::Retired(retired) => {
            settle(permit, &writer, retired.seal().clone(), runtime, admission)
        }
        CodexRetirementState::Sealed(seal) => settle(permit, &writer, seal, runtime, admission),
        CodexRetirementState::Unsealed => {
            let Some(gone) = gone else {
                return Ok(pending());
            };
            let seal = CodexRetirementSeal::capture(&admission, gone)?;
            let publication = writer.publish_seal(&seal, &admission)?;
            #[cfg(test)]
            test_fault::run(&runtime, RetirementFaultStage::AfterSealPublication)?;
            settle(
                permit,
                &writer,
                publication.artifact().clone(),
                runtime,
                admission,
            )
        }
    }
}

fn settle(
    permit: CodexRetirementPermit,
    writer: &CodexRetirementWriterFence,
    seal: CodexRetirementSealArtifact,
    runtime: Arc<CodexHookRuntime>,
    admission: CodexAdmissionAuthority,
) -> Result<IntakeRouteReport> {
    let intake = writer.open_terminal_intake(&seal, &admission)?;
    let authority = CodexRetirementIntakeAuthority::new(&runtime, &permit, writer, &seal)
        .map_err(anyhow::Error::new)?;
    let report = runtime.reconcile_retirement_intake(&authority, &intake)?;
    if report.deferred != usize::default() {
        return Ok(IntakeRouteReport::RetirementPending(report));
    }
    let prepared = runtime.prepare_retirement(writer, &seal, &admission)?;
    #[cfg(test)]
    test_fault::run(&runtime, RetirementFaultStage::AfterTerminalSettlement)?;
    #[cfg(test)]
    test_fault::run(&runtime, RetirementFaultStage::BeforeTerminalIntakeProof)?;
    let intake_proof = writer.prove_terminal_intake(&seal, &admission)?;
    let publication = writer.publish_retirement(
        &prepared.record,
        &seal,
        &prepared.snapshot,
        &admission,
        &prepared.proof,
        &intake_proof,
        &prepared.outbox,
    )?;
    #[cfg(test)]
    test_fault::run(&runtime, RetirementFaultStage::AfterRetirementPublication)?;
    writer.verify_retirement_drained(
        publication.artifact(),
        &seal,
        &prepared.snapshot,
        &admission,
        &prepared.proof,
        &intake_proof,
        &prepared.outbox,
    )?;
    permit.verify().map_err(anyhow::Error::new)?;
    permit.commit().map_err(anyhow::Error::new)?;
    Ok(IntakeRouteReport::Retired(report))
}

fn pending() -> IntakeRouteReport {
    IntakeRouteReport::RetirementPending(Default::default())
}

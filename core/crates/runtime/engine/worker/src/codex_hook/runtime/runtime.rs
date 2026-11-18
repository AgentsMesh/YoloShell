use super::authority_registry::CodexAuthorityRegistration;
#[cfg(test)]
use super::error::CodexHookRejection;
use super::publication::CodexAuthorityView;
pub(super) use super::publication::CodexHookPublication;
#[cfg(test)]
pub(super) use super::publication::CodexProcessExitPublication;
pub(in crate::worker) use super::publication::ReplayReport;
use crate::agent::ProcessSource;
use crate::agent::hooks::{HookEnvelope, HookIdentity};
#[cfg(test)]
use anyhow::Result;
use std::sync::Arc;
use uuid::Uuid;

pub(super) enum CodexIngressEvidence<'a> {
    #[cfg(test)]
    TestPeer(u32),
    VerifiedDurableOccurrence(intake_admission::VerifiedDurableOccurrence),
    VerifiedRetirementOccurrence(retirement_intake_admission::VerifiedRetirementOccurrence<'a>),
}

#[cfg(test)]
#[cfg(any(not(test), feature = "yolo-test-support"))]
#[path = "../../../test_support/unit/codex_hook/runtime/runtime_ack_test_support.rs"]
mod ack_test_support;
#[path = "runtime_build.rs"]
mod build;
#[path = "runtime_build_support.rs"]
mod build_support;
#[path = "runtime_control.rs"]
mod control;
#[path = "runtime_ingest.rs"]
mod ingest;
#[path = "runtime_intake.rs"]
mod intake;
#[path = "runtime_intake_admission.rs"]
mod intake_admission;
#[path = "runtime_intake_consume.rs"]
mod intake_consume;
#[cfg(test)]
#[cfg(any(not(test), feature = "yolo-test-support"))]
#[path = "../../../test_support/unit/codex_hook/runtime/runtime_intake_test_gate.rs"]
mod intake_test_gate;
#[path = "runtime_live.rs"]
mod live;
#[path = "runtime_live_failure.rs"]
mod live_failure;
#[path = "runtime_live_occurrence.rs"]
mod live_occurrence;
#[path = "runtime_live_producer.rs"]
mod live_producer;
#[path = "runtime_live_settlement.rs"]
mod live_settlement;
#[path = "runtime_lock.rs"]
mod locking;
#[path = "runtime_native_intake.rs"]
mod native_intake;
#[path = "runtime_native_registration.rs"]
mod native_registration;
#[path = "runtime_retirement.rs"]
mod retirement;
#[path = "runtime_retirement_intake.rs"]
mod retirement_intake;
#[path = "runtime_retirement_intake_admission.rs"]
mod retirement_intake_admission;
#[path = "runtime_authority.rs"]
mod runtime_authority;
#[path = "runtime_view.rs"]
mod runtime_view;
#[path = "runtime_takeover.rs"]
mod takeover;
#[path = "runtime_terminal.rs"]
mod terminal;
#[cfg(test)]
#[cfg(any(not(test), feature = "yolo-test-support"))]
#[path = "../../../test_support/unit/codex_hook/runtime/runtime_test_state.rs"]
mod test_state;
#[cfg(test)]
#[cfg(any(not(test), feature = "yolo-test-support"))]
#[path = "../../../test_support/unit/codex_hook/runtime/runtime_test_support.rs"]
mod test_support;
#[cfg(test)]
pub(in crate::worker::codex_hook) use build_support::set_before_initial_publish_hook;
pub(in crate::worker::codex_hook) use intake::IntakeReplayReport;
#[cfg(test)]
pub(in crate::worker::codex_hook) use intake_admission::VerifiedDurableOccurrence;
pub(in crate::worker::codex_hook) use live::{CodexLiveHandoffOutcome, CodexLiveIngest};
pub(in crate::worker::codex_hook) use live_failure::CodexLiveFailure;
use live_occurrence::VerifiedLiveOccurrence;
pub(in crate::worker::codex_hook) use live_producer::CodexLiveProducer;
pub(in crate::worker::codex_hook) use live_settlement::CodexLiveSettlement;
pub(in crate::worker::codex_hook) use retirement_intake_admission::CodexRetirementIntakeAuthority;
#[cfg(test)]
pub(in crate::worker::codex_hook) use test_state::RuntimeTestKey;

pub(in crate::worker) struct CodexHookRuntime {
    registration: CodexAuthorityRegistration,
    pub(super) terminal_id: Uuid,
    pub(super) runtime_epoch: Uuid,
    pub(super) shell: crate::process::ProcessIdentity,
    owner: HookIdentity,
    artifacts: crate::codex_hooks::CodexHookArtifacts,
    outbox: crate::codex_hooks::CodexOutbox,
    pub(super) process_source: Arc<dyn ProcessSource>,
    #[cfg(test)]
    faults: test_state::RuntimeFaults,
}

struct CommittedLiveEvent {
    event: HookEnvelope,
    occurrence: Option<VerifiedLiveOccurrence>,
    durable_rejection: Option<crate::codex_hooks::CodexDurableRejection>,
    generation_fence: super::cancellation::IngressGenerationFence,
}

impl CodexHookRuntime {
    #[cfg(test)]
    pub(super) fn ingest(
        &self,
        event: HookEnvelope,
        peer_pid: u32,
    ) -> Result<CodexHookPublication, CodexHookRejection> {
        let occurrence_identity = self.test_occurrence_identity(&event)?;
        let mut lifecycle = self.lock_lifecycle()?;
        let evidence = CodexIngressEvidence::TestPeer(peer_pid);
        let committed = self.ingest_locked(
            &mut lifecycle,
            event,
            &evidence,
            occurrence_identity.as_ref(),
        )?;
        match committed.rejection {
            Some(violation) => Err(CodexHookRejection::Violation(violation)),
            None => Ok(committed.publication),
        }
    }

    #[cfg(test)]
    pub(in crate::worker::codex_hook) fn ingest_with_handoff(
        &self,
        event: HookEnvelope,
        peer_pid: u32,
        handoff: impl FnOnce(Result<CodexHookPublication, CodexHookRejection>) -> bool,
    ) {
        let occurrence_identity = match self.test_occurrence_identity(&event) {
            Ok(identity) => identity,
            Err(error) => {
                let _ = handoff(Err(error));
                return;
            }
        };
        let mut lifecycle = match self.lock_lifecycle() {
            Ok(lifecycle) => lifecycle,
            Err(error) => {
                let _ = handoff(Err(error));
                return;
            }
        };
        let evidence = CodexIngressEvidence::TestPeer(peer_pid);
        match self.ingest_locked(
            &mut lifecycle,
            event,
            &evidence,
            occurrence_identity.as_ref(),
        ) {
            Ok(committed) if committed.rejection.is_some() => {
                let violation = committed.rejection.expect("checked rejection");
                let _ = handoff(Err(CodexHookRejection::Violation(violation)));
            }
            Ok(committed) => {
                if !handoff(Ok(committed.publication)) {
                    lifecycle.latch_storage_failure();
                    self.publish_storage_unavailable();
                }
            }
            Err(error) => {
                let authority_lost = matches!(&error, CodexHookRejection::AuthorityLost(_));
                if !handoff(Err(error)) && !authority_lost {
                    lifecycle.latch_storage_failure();
                    self.publish_storage_unavailable();
                }
            }
        }
    }
}

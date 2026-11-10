use super::pty_output_progress::{
    ExclusiveOutputQuiescenceFence, ExclusivePtyOutput, KernelPtyEofAuthority,
    OutputQuiescenceFence, PtyOutputRecovery,
};
use super::pty_process_custody::PtyProcessCustody;
use crate::process::{
    FreezeReady, FrozenProducersGoneReady, KillCommitReady, KillPollReady, ProcessClosureVersion,
    ProducersGoneReportReady, SurvivorReady,
};
use anyhow::{Context, Result};

#[path = "pty_force_termination_commit.rs"]
mod commit;
#[path = "pty_force_termination_empty_closure.rs"]
mod empty_closure;
#[path = "pty_force_termination_empty_commit.rs"]
mod empty_commit;
use empty_closure::EmptyClosureOutputRequirement;
#[path = "pty_force_termination_fence.rs"]
mod fence;
#[path = "pty_force_termination_master_release.rs"]
mod master_release;
#[path = "pty_force_termination_output.rs"]
mod output;
#[path = "pty_force_termination_prepare.rs"]
mod prepare;
#[path = "pty_force_termination_progress.rs"]
mod progress;
pub(crate) use progress::ForceTransactionState;
#[path = "pty_force_termination_reported.rs"]
mod reported;
use reported::ReportedProgressReady;

pub(crate) const PROCESS_CUSTODY_MISSING_CONTEXT: &str = "PTY process custody is unavailable";
pub(crate) const OUTPUT_FENCE_KIND_CHANGED_DETAIL: &str = "PTY force-output fence kind changed";

#[cfg(all(test, feature = "yolo-test-support"))]
include!("../../../test_support/integration/pty/pty_force_output_test_support.rs");
#[cfg(all(test, feature = "yolo-test-support"))]
include!("../../../test_support/integration/pty/pty_force_output_eof_test_support.rs");

pub(crate) enum ForceTerminationPoll {
    Pending,
    Complete,
    Unsupported(crate::process::DestructiveSessionUnavailable),
}

pub(crate) enum OutputFence {
    Reader(OutputQuiescenceFence),
    Exclusive(ExclusiveOutputQuiescenceFence),
    #[cfg(test)]
    Scripted,
    Reached,
}

pub(crate) struct VersionedOutputFence {
    version: ProcessClosureVersion,
    state: OutputFence,
    kernel_eof: Option<KernelPtyEofAuthority>,
}

pub(crate) enum ForceReady {
    Unchanged,
    Tracking,
    Freezing(FreezeReady),
    Frozen(KillCommitReady),
    FrozenGone,
    Killing(KillPollReady),
    Survivor(SurvivorReady),
    GonePending(ProducersGoneReportReady<()>),
    GoneReported(ReportedProgressReady),
    MasterClosed(Option<crate::process::PtyMasterAliasesClosed>),
}

pub(crate) enum EmptyGoneOutputAuthority {
    NotRequired,
    Kernel(KernelPtyEofAuthority),
}

pub(crate) struct FrozenGoneReady {
    process: FrozenProducersGoneReady,
    output: EmptyGoneOutputAuthority,
}

pub(crate) enum ForceOutput<'output> {
    Reader(&'output mut PtyOutputRecovery),
    Exclusive(&'output mut ExclusivePtyOutput),
    #[cfg(test)]
    Scripted(&'output mut ScriptedForceOutput),
}

pub(crate) struct PtyForceTermination {
    pub(crate) force_requested: bool,
    pub(crate) transaction_state: ForceTransactionState,
    pub(crate) output_fence: Option<VersionedOutputFence>,
    pub(crate) frozen_gone: Option<FrozenGoneReady>,
    pub(crate) progressed: bool,
    #[cfg(test)]
    pub(crate) empty_closure_output_requirement: Option<EmptyClosureOutputRequirement>,
}

impl PtyForceTermination {
    pub(crate) fn new() -> Self {
        Self {
            force_requested: false,
            transaction_state: ForceTransactionState::Unstarted,
            output_fence: None,
            frozen_gone: None,
            progressed: false,
            #[cfg(test)]
            empty_closure_output_requirement: None,
        }
    }

    pub(crate) fn poll(
        &mut self,
        custody: &mut Option<PtyProcessCustody>,
        output: &mut ForceOutput<'_>,
    ) -> Result<ForceTerminationPoll> {
        self.begin_transaction();
        let state = custody.as_mut().context(PROCESS_CUSTODY_MISSING_CONTEXT)?;
        let ready = match self.prepare(state, output) {
            Ok(ready) => ready,
            Err(error) => {
                let proof = transition_unsupported(custody, error)?;
                self.output_fence = None;
                return Ok(ForceTerminationPoll::Unsupported(proof));
            }
        };
        self.commit(custody, output, ready)
    }
}

pub(crate) fn transition_unsupported(
    custody: &mut Option<PtyProcessCustody>,
    error: anyhow::Error,
) -> Result<crate::process::DestructiveSessionUnavailable> {
    let proof = match error.downcast::<crate::process::DestructiveSessionUnavailable>() {
        Ok(proof) => proof,
        Err(error) => return Err(error),
    };
    let state = custody.take().context(PROCESS_CUSTODY_MISSING_CONTEXT)?;
    let survivor = match state {
        PtyProcessCustody::Tracking(tree) => tree.into_termination_unsupported(&proof),
        PtyProcessCustody::Freezing(tree) => match tree.into_termination_unsupported(&proof) {
            Ok(tree) => tree,
            Err(failure) => {
                let (tree, error) = failure.into_parts();
                *custody = Some(PtyProcessCustody::Freezing(tree));
                return Err(error);
            }
        },
        state => {
            *custody = Some(state);
            return Err(anyhow::Error::new(proof));
        }
    };
    *custody = Some(PtyProcessCustody::Survivor(survivor));
    Ok(proof)
}

#[cfg(test)]
#[cfg(any(not(test), feature = "yolo-integration-tests"))]
#[path = "../../../integration_tests/pty/force/pty_force_termination_tests.rs"]
mod tests;

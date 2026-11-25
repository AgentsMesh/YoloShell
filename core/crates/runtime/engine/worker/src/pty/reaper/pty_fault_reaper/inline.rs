use super::{FaultOutput, FaultReapCompletion, ReapOutcome};
use crate::process::{CapturedProcessTree, RootWaitAuthority};
use crate::worker::pty_force_termination::PtyForceTermination;
use crate::worker::pty_output_progress::{ExclusivePtyOutput, ReaderBackedPtyOutput};
use crate::worker::pty_process_custody::PtyProcessCustody;

pub(crate) fn settle_inline(
    authority: RootWaitAuthority,
    tree: CapturedProcessTree,
    output_progress: ReaderBackedPtyOutput,
    failure: Option<anyhow::Error>,
    state_observer: super::FaultReapStateObserver,
) -> ReapOutcome {
    reap(
        authority,
        tree,
        FaultOutput::Reader(output_progress),
        failure,
        state_observer,
    )
}

pub(crate) fn settle_inline_after_master_release(
    authority: RootWaitAuthority,
    tree: CapturedProcessTree,
    output: ExclusivePtyOutput,
    failure: Option<anyhow::Error>,
    state_observer: super::FaultReapStateObserver,
) -> ReapOutcome {
    reap(
        authority,
        tree,
        FaultOutput::Exclusive(output),
        failure,
        state_observer,
    )
}

pub(crate) fn reap(
    authority: RootWaitAuthority,
    tree: CapturedProcessTree,
    output_progress: FaultOutput,
    failure: Option<anyhow::Error>,
    state_observer: super::FaultReapStateObserver,
) -> ReapOutcome {
    let (completion, receiver) = FaultReapCompletion::owner_channel();
    let ownership = super::fault_reap_construction::fault_reap(
        authority,
        PtyProcessCustody::tracking(tree),
        PtyForceTermination::new(),
        output_progress,
        crate::worker::pty_missing_eof::MissingEofDeadline::production(),
        None,
        failure,
        completion,
        state_observer,
    );
    ownership.publish_handoff();
    ownership.reap();
    receiver.receive()
}

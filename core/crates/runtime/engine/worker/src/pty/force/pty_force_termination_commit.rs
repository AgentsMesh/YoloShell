use super::{
    ForceOutput, ForceReady, ForceTerminationPoll, PtyForceTermination, ReportedProgressReady,
};
use crate::process::{
    FreezePoll, KillCommitPoll, KillPoll, ProcessTransitionError, ReportedProducersGone,
    ReportedProducersGoneRetirement, SurvivorPoll,
};
use crate::worker::pty_process_custody::PtyProcessCustody;
use anyhow::Result;

pub(crate) const PROCESS_CUSTODY_COMMIT_MISSING_DETAIL: &str =
    "PTY process custody disappeared before a prepared transition";
pub(crate) const PROCESS_CUSTODY_VARIANT_CHANGED_DETAIL: &str =
    "PTY process custody changed after a prepared transition";

impl PtyForceTermination {
    pub(crate) fn commit(
        &mut self,
        custody: &mut Option<PtyProcessCustody>,
        output: &mut ForceOutput<'_>,
        ready: ForceReady,
    ) -> Result<ForceTerminationPoll> {
        if matches!(ready, ForceReady::Unchanged) {
            return Ok(ForceTerminationPoll::Pending);
        }
        if matches!(ready, ForceReady::FrozenGone) {
            return self.commit_frozen_gone(custody, output);
        }
        if let ForceReady::MasterClosed(proof) = ready {
            return super::master_release::commit_master_closed(custody, proof);
        }
        let state = custody.take().expect(PROCESS_CUSTODY_COMMIT_MISSING_DETAIL);
        let (next, poll) = match (state, ready) {
            (PtyProcessCustody::Tracking(tree), ForceReady::Tracking) => (
                PtyProcessCustody::Freezing(tree.begin_force()),
                ForceTerminationPoll::Pending,
            ),
            (PtyProcessCustody::Freezing(tree), ForceReady::Freezing(ready)) => {
                let next = match tree.commit_poll(ready) {
                    FreezePoll::Pending(tree) => PtyProcessCustody::Freezing(tree),
                    FreezePoll::Frozen(tree) => PtyProcessCustody::Frozen(tree),
                };
                (next, ForceTerminationPoll::Pending)
            }
            (PtyProcessCustody::Frozen(tree), ForceReady::Frozen(ready)) => {
                let next = match tree.commit_kill(ready) {
                    KillCommitPoll::Refreeze(tree) => PtyProcessCustody::Freezing(tree),
                    KillCommitPoll::Pending(tree) => PtyProcessCustody::Killing(tree),
                };
                (next, ForceTerminationPoll::Pending)
            }
            (PtyProcessCustody::Killing(tree), ForceReady::Killing(ready)) => {
                let next = match tree.commit_poll(ready) {
                    KillPoll::Refreeze(tree) => PtyProcessCustody::Freezing(tree),
                    KillPoll::Pending(tree) => PtyProcessCustody::Killing(tree),
                    KillPoll::Gone(completion) => PtyProcessCustody::GonePending(completion),
                };
                (next, ForceTerminationPoll::Pending)
            }
            (PtyProcessCustody::Survivor(tree), ForceReady::Survivor(ready)) => {
                let next = match tree.commit_poll(ready) {
                    SurvivorPoll::Pending(tree) => PtyProcessCustody::Survivor(tree),
                    SurvivorPoll::Gone(completion) => PtyProcessCustody::GonePending(completion),
                };
                (next, ForceTerminationPoll::Pending)
            }
            (PtyProcessCustody::GonePending(completion), ForceReady::GonePending(ready)) => {
                let (completion, ()) = completion.commit_report(ready);
                (
                    PtyProcessCustody::GoneReported(completion),
                    ForceTerminationPoll::Pending,
                )
            }
            (PtyProcessCustody::GoneReported(completion), ForceReady::GoneReported(ready)) => {
                match commit_reported_progress(completion, ready) {
                    Ok(committed) => committed,
                    Err(failure) => {
                        let (completion, error) = failure.into_parts();
                        *custody = Some(PtyProcessCustody::GoneReported(completion));
                        return Err(error);
                    }
                }
            }
            (state, _) => {
                *custody = Some(state);
                panic!("{PROCESS_CUSTODY_VARIANT_CHANGED_DETAIL}");
            }
        };
        *custody = Some(next);
        Ok(poll)
    }
}

pub(crate) fn commit_reported_progress(
    completion: ReportedProducersGone,
    ready: ReportedProgressReady,
) -> std::result::Result<
    (PtyProcessCustody, ForceTerminationPoll),
    ProcessTransitionError<ReportedProducersGone>,
> {
    let ReportedProgressReady::Retirement(ready) = ready else {
        return Ok((
            PtyProcessCustody::GoneReported(completion),
            ForceTerminationPoll::Pending,
        ));
    };
    Ok(match completion.into_retirement(ready)? {
        ReportedProducersGoneRetirement::Direct(ready) => (
            PtyProcessCustody::GoneReported(ready.commit()),
            ForceTerminationPoll::Complete,
        ),
        ReportedProducersGoneRetirement::MasterRelease(completion) => (
            PtyProcessCustody::MasterClosePending(completion),
            ForceTerminationPoll::Pending,
        ),
    })
}

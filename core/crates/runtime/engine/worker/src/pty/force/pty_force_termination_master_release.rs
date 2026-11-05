use super::{ForceOutput, ForceTerminationPoll};
use crate::process::ProducersGoneRetirement;
use crate::worker::pty_output_progress::MasterAliasReleasePoll;
use crate::worker::pty_process_custody::PtyProcessCustody;
use anyhow::Result;

pub(crate) const PROCESS_CUSTODY_COMMIT_MISSING_DETAIL: &str =
    "PTY process custody disappeared before a prepared transition";
pub(crate) const PROCESS_CUSTODY_VARIANT_CHANGED_DETAIL: &str =
    "PTY process custody changed after a prepared transition";

impl ForceOutput<'_> {
    pub(crate) fn release_master_aliases(&mut self) -> Result<MasterAliasReleasePoll> {
        match self {
            Self::Reader(output) => output.release_master_aliases(),
            Self::Exclusive(output) => output.release_master_aliases(),
            #[cfg(test)]
            Self::Scripted(output) if output.master_release_delivered => {
                Ok(MasterAliasReleasePoll::Delivered)
            }
            #[cfg(test)]
            Self::Scripted(output) => {
                output.master_release_delivered = true;
                Ok(MasterAliasReleasePoll::Closed(
                    crate::process::PtyMasterAliasesClosed::scripted(),
                ))
            }
        }
    }
}

pub(crate) fn commit_master_closed(
    custody: &mut Option<PtyProcessCustody>,
    proof: Option<crate::process::PtyMasterAliasesClosed>,
) -> Result<ForceTerminationPoll> {
    let ready = match custody.as_mut() {
        Some(PtyProcessCustody::MasterClosePending(completion)) => {
            completion.prepare_retirement_after_master_release(proof)?
        }
        Some(_) => panic!("{PROCESS_CUSTODY_VARIANT_CHANGED_DETAIL}"),
        None => panic!("{PROCESS_CUSTODY_COMMIT_MISSING_DETAIL}"),
    };
    let state = custody.take().expect(PROCESS_CUSTODY_COMMIT_MISSING_DETAIL);
    let PtyProcessCustody::MasterClosePending(completion) = state else {
        unreachable!("{PROCESS_CUSTODY_VARIANT_CHANGED_DETAIL}");
    };
    let retirement = match completion.commit_retirement_after_master_release(ready) {
        Ok(retirement) => retirement,
        Err(failure) => {
            let (completion, error) = failure.into_parts();
            *custody = Some(PtyProcessCustody::MasterClosePending(completion));
            return Err(error);
        }
    };
    let (next, poll) = match retirement {
        ProducersGoneRetirement::Refreeze(tree) => (
            PtyProcessCustody::Freezing(tree),
            ForceTerminationPoll::Pending,
        ),
        ProducersGoneRetirement::Retired(completion) => (
            PtyProcessCustody::GoneReported(completion),
            ForceTerminationPoll::Complete,
        ),
    };
    *custody = Some(next);
    Ok(poll)
}

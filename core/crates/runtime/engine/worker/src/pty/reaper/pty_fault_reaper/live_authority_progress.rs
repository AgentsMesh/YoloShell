use super::super::{FaultOutput, FaultReap};
use crate::worker::pty_force_termination::{ForceOutput, ForceTerminationPoll};
use crate::worker::pty_missing_eof::{MissingEofPoll, poll as poll_missing_eof};
use crate::worker::pty_ownership::PtyOwnershipCustody;
use std::time::Instant;

pub(crate) fn poll_force(ownership: &mut FaultReap) -> anyhow::Result<ForceTerminationPoll> {
    let custody: &mut PtyOwnershipCustody = ownership.custody_mut();
    let PtyOwnershipCustody {
        processes,
        force,
        output_progress,
        ..
    } = custody;
    match output_progress {
        FaultOutput::Reader(output) => {
            force.poll(processes, &mut ForceOutput::Reader(&mut output.recovery()))
        }
        FaultOutput::Exclusive(output) => {
            force.poll(processes, &mut ForceOutput::Exclusive(output))
        }
    }
}

pub(crate) fn poll_output_deadline(ownership: &mut FaultReap) -> anyhow::Result<MissingEofPoll> {
    if ownership.is_observer_only() {
        return Ok(MissingEofPoll::Pending);
    }
    let custody: &mut PtyOwnershipCustody = ownership.custody_mut();
    let PtyOwnershipCustody {
        processes,
        output_progress,
        missing_eof,
        ..
    } = custody;
    let custody = processes.as_mut().expect(PROCESS_CUSTODY_MISSING_DETAIL);
    match output_progress {
        FaultOutput::Reader(output) => {
            poll_missing_eof(missing_eof, custody, &mut output.recovery(), Instant::now())
        }
        FaultOutput::Exclusive(output) => {
            poll_missing_eof(missing_eof, custody, output, Instant::now())
        }
    }
}

pub(crate) const PROCESS_CUSTODY_MISSING_DETAIL: &str = "PTY process custody is present";

pub(crate) fn finish_if_ready(ownership: &mut FaultReap) -> bool {
    if ownership
        .custody()
        .processes
        .as_ref()
        .is_some_and(|custody| !custody.session_is_retired())
    {
        return false;
    }
    if ownership.custody().authority.is_none() {
        if ownership.root_gone_without_wait {
            ownership.finish_lost();
            return true;
        }
        return false;
    }
    let Some(proof) = ownership.custody_mut().root_exit_proof.take() else {
        return false;
    };
    ownership.finish_reaped(proof)
}

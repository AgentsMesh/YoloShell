use super::{FaultOutput, FaultReap};
use crate::worker::pty_output_progress::MasterAliasReleasePoll;
use crate::worker::pty_ownership::PtyOwnershipCustody;
use anyhow::{Result, bail};

pub(crate) const RELEASE_PROOF_ALREADY_DELIVERED_DETAIL: &str =
    "explicit bootstrap failure lost its PTY master-closure proof";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum BootstrapFailurePoll {
    NotApplicable,
    Pending,
    Retired,
}

pub(crate) fn poll(ownership: &mut FaultReap) -> Result<BootstrapFailurePoll> {
    let custody: &mut PtyOwnershipCustody = ownership.custody_mut();
    let PtyOwnershipCustody {
        processes,
        output_progress,
        root_exit_proof,
        ..
    } = custody;
    let Some(tree) = processes
        .as_mut()
        .and_then(crate::worker::pty_process_custody::PtyProcessCustody::try_tracking_mut)
    else {
        return Ok(BootstrapFailurePoll::NotApplicable);
    };
    if !tree.explicit_bootstrap_exec_failure_is_pending() {
        return Ok(BootstrapFailurePoll::NotApplicable);
    }
    if root_exit_proof.is_none() {
        return Ok(BootstrapFailurePoll::Pending);
    }
    let release = match output_progress {
        FaultOutput::Reader(output) => output.recovery().release_master_aliases()?,
        FaultOutput::Exclusive(output) => output.release_master_aliases()?,
    };
    match release {
        MasterAliasReleasePoll::Pending => Ok(BootstrapFailurePoll::Pending),
        MasterAliasReleasePoll::Closed(proof) => {
            tree.prepare_explicit_bootstrap_failure_retirement(proof)?
                .commit();
            Ok(BootstrapFailurePoll::Retired)
        }
        MasterAliasReleasePoll::Delivered => bail!(RELEASE_PROOF_ALREADY_DELIVERED_DETAIL),
    }
}

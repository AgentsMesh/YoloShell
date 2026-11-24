use super::{LiveProcessIdentity, ProcessExecutionState};
use anyhow::Result;

pub(super) fn execution_state(identity: LiveProcessIdentity) -> Result<ProcessExecutionState> {
    let Some(before) = exact_metadata(identity)? else {
        return Ok(ProcessExecutionState::Gone);
    };
    let Some(after) = exact_metadata(identity)? else {
        return Ok(ProcessExecutionState::Gone);
    };
    Ok(
        if before.status == libc::SSTOP && after.status == libc::SSTOP {
            ProcessExecutionState::Stopped
        } else {
            ProcessExecutionState::Running
        },
    )
}

fn exact_metadata(
    identity: LiveProcessIdentity,
) -> Result<Option<crate::process::macos_kernel::KernelProcessMetadata>> {
    Ok(crate::process::macos_kernel::metadata(identity.pid())?
        .filter(|metadata| metadata.unique_id == identity.unique_id()))
}

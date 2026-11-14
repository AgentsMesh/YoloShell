use crate::model::{ExitReason, TerminalRuntimeStatus, TerminalView};
use crate::protocol::ProtocolError;

pub(super) fn expected_barrier(session: &TerminalView) -> Result<Option<u64>, ProtocolError> {
    let Some(completed) = session.completed_attachment else {
        return Ok(None);
    };
    let runtime = session
        .runtime
        .as_ref()
        .ok_or(ProtocolError::AttachmentMismatch)?;
    let exit = runtime
        .exit
        .as_ref()
        .ok_or(ProtocolError::AttachmentMismatch)?;
    if !session.manifest.archived
        && completed.runtime_epoch == runtime.runtime_epoch
        && runtime.status == TerminalRuntimeStatus::Exited
        && exit.runtime_epoch == runtime.runtime_epoch
        && exit.reason == ExitReason::Natural
        && exit.last_output_sequence == runtime.last_output_sequence
    {
        return Ok(Some(exit.last_output_sequence));
    }
    Err(ProtocolError::AttachmentMismatch)
}

#[cfg(test)]
#[cfg(any(not(test), feature = "yolo-unit-tests"))]
#[path = "../unit_tests/completed_snapshot_tests.rs"]
mod tests;

use super::RelayError;
use super::constants::TERMINAL_RESET;
use crate::model::OutputChunk;
use crate::protocol::{EventEnvelope, ProtocolError, event, from_value, validate_attachment_event};
use crate::worker::{AttachResult, attachment_next_sequence};
use tokio::io::{AsyncWrite, AsyncWriteExt};

#[derive(Debug, Eq, PartialEq)]
pub(super) enum EventAction {
    Continue,
    Reattach,
    SessionExited,
}

pub(super) async fn write_initial<W: AsyncWrite + Unpin + ?Sized>(
    output: &mut W,
    initial: &AttachResult,
    after_sequence: u64,
    reset: bool,
) -> Result<u64, RelayError> {
    let observed_sequence = initial.validate_for_request(initial.runtime_epoch, after_sequence)?;
    if initial.snapshot.is_some() && (reset || initial.resynchronized) {
        output.write_all(TERMINAL_RESET).await?;
    }
    if let Some(snapshot) = &initial.snapshot {
        output.write_all(&snapshot.data).await?;
    }
    for chunk in &initial.chunks {
        output.write_all(&chunk.data).await?;
    }
    output.flush().await?;
    Ok(observed_sequence)
}

pub(super) async fn write_event<W: AsyncWrite + Unpin + ?Sized>(
    output: &mut W,
    envelope: &EventEnvelope,
    terminal_id: uuid::Uuid,
    runtime_epoch: uuid::Uuid,
    sequence: &mut u64,
) -> Result<EventAction, RelayError> {
    validate_attachment_event(envelope, terminal_id, runtime_epoch)?;
    match envelope.event.as_str() {
        event::TERMINAL_OUTPUT => {
            let chunk: OutputChunk = from_value(envelope.payload.clone())?;
            if chunk.runtime_epoch != runtime_epoch {
                return Err(ProtocolError::EventMismatch.into());
            }
            if chunk.sequence <= *sequence {
                return Ok(EventAction::Continue);
            }
            if attachment_next_sequence(*sequence) != Some(chunk.sequence) {
                return Ok(EventAction::Reattach);
            }
            output.write_all(&chunk.data).await?;
            *sequence = chunk.sequence;
            Ok(EventAction::Continue)
        }
        event::OUTPUT_GAP => Ok(EventAction::Reattach),
        event::SESSION_EXITED => {
            let exit: crate::model::ExitRecord = from_value(envelope.payload.clone())?;
            if exit.last_output_sequence != *sequence {
                return Err(ProtocolError::EventMismatch.into());
            }
            Ok(EventAction::SessionExited)
        }
        _ => Ok(EventAction::Continue),
    }
}

use crate::{attachment::chunk_from, errors::map_error, protocol};
use yolo_client::ClientError;
use yolo_hub_identity::RuntimeTarget;
use yolo_hub_terminal_protocol::{OutputCursor, PublicError, TerminalEvent};
use yolo_terminal_model::{ExitRecord, OutputGapPayload};
use yolo_wire::{self, EventEnvelope, ProtocolError};

pub(crate) fn convert_event(
    target: RuntimeTarget,
    event: EventEnvelope,
) -> Result<Option<TerminalEvent>, PublicError> {
    protocol::validate_attachment_event(
        &event,
        target.address.terminal_id.as_uuid(),
        target.runtime_epoch.as_uuid(),
    )
    .map_err(|_| PublicError::TargetMismatch)?;
    match event.event.as_str() {
        protocol::event::TERMINAL_OUTPUT => {
            let chunk =
                protocol::from_value(event.payload).map_err(|_| PublicError::InvalidPayload)?;
            Ok(Some(TerminalEvent::Output {
                chunk: chunk_from(target, &chunk)?,
            }))
        }
        protocol::event::OUTPUT_GAP => {
            let gap: OutputGapPayload =
                protocol::from_value(event.payload).map_err(|_| PublicError::InvalidPayload)?;
            if gap.skipped == 0 || !gap.resync_required {
                return Err(PublicError::InvalidPayload);
            }
            Ok(Some(TerminalEvent::ResyncRequired { target }))
        }
        protocol::event::SESSION_EXITED => {
            let exit: ExitRecord =
                protocol::from_value(event.payload).map_err(|_| PublicError::InvalidPayload)?;
            if exit.runtime_epoch != target.runtime_epoch.as_uuid() {
                return Err(PublicError::TargetMismatch);
            }
            let event = TerminalEvent::Exited {
                cursor: OutputCursor {
                    target,
                    after_sequence: exit.last_output_sequence,
                },
                code: exit.code,
                signal: exit.signal,
            };
            event.validate()?;
            Ok(Some(event))
        }
        _ => Ok(None),
    }
}

pub(crate) fn connection_loss(
    target: RuntimeTarget,
    error: ClientError,
) -> Result<TerminalEvent, PublicError> {
    match error {
        ClientError::Protocol(ProtocolError::EndOfStream | ProtocolError::Io(_))
        | ClientError::RuntimeUnavailable { .. }
        | ClientError::Deadline => Ok(TerminalEvent::Unreachable { target }),
        error => Err(map_error(error)),
    }
}

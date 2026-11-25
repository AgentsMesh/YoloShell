use super::{
    AttachmentRecovery, RelayCancellation, RelayError, RelayExit, RelayOptions, RelayRuntime,
    reconnect_runtime, wait_to_reconnect, write_initial,
};
use crate::model::ExitRecord;
use crate::protocol::{ProtocolError, event, from_value};
use crate::relay::output::{EventAction, write_event};
use crate::relay::status::{RelayStatusSink, RelayStatusTarget};
use crate::relay::{RelayStatus, client_error_is_retryable};
use tokio::io::{AsyncWrite, AsyncWriteExt};
use uuid::Uuid;

pub(super) async fn relay_completed<W: AsyncWrite + Unpin + Send>(
    runtime: &RelayRuntime,
    options: &RelayOptions,
    output: &mut W,
    runtime_epoch: Uuid,
    expected_exit: ExitRecord,
    status: &RelayStatusSink,
    cancellation: &RelayCancellation,
) -> Result<RelayExit, RelayError> {
    let mut sequence = options.after_sequence;
    let mut reset = false;
    let mut ending_announced = false;
    let recovery = AttachmentRecovery::Completed {
        runtime_epoch,
        expected_exit: &expected_exit,
    };
    loop {
        cancellation.check()?;
        let mut attachment = super::attach_with_recovery(
            runtime,
            options.terminal_id,
            sequence,
            recovery,
            cancellation,
        )
        .await?;
        if attachment.runtime_epoch() != runtime_epoch {
            return Err(RelayError::RuntimeEpochChanged {
                expected: runtime_epoch,
                actual: attachment.runtime_epoch(),
            });
        }
        if !ending_announced {
            status.send_status(RelayStatus::SessionEnding);
            ending_announced = true;
        }
        sequence = cancellation
            .wait(write_initial(output, &attachment.initial, sequence, reset))
            .await??;
        loop {
            let event = match cancellation.wait(attachment.next_event()).await? {
                Ok(event) => event,
                Err(error)
                    if client_error_is_retryable(&error)
                        || crate::relay::host::observation_interrupted(&error) =>
                {
                    reconnect_runtime(runtime, options.terminal_id, recovery, cancellation).await?;
                    wait_to_reconnect(cancellation).await?;
                    reset = true;
                    break;
                }
                Err(error) => return Err(error.into()),
            };
            let action = cancellation
                .wait(write_event(
                    output,
                    &event,
                    options.terminal_id,
                    runtime_epoch,
                    &mut sequence,
                ))
                .await??;
            match action {
                EventAction::Continue => {}
                EventAction::Reattach => {
                    reconnect_runtime(runtime, options.terminal_id, recovery, cancellation).await?;
                    reset = true;
                    break;
                }
                EventAction::SessionExited => {
                    let exit: ExitRecord = from_value(event.payload)?;
                    if event.event != event::SESSION_EXITED
                        || exit != expected_exit
                        || sequence != exit.last_output_sequence
                    {
                        return Err(ProtocolError::EventMismatch.into());
                    }
                    cancellation.wait(output.flush()).await??;
                    return Ok(RelayExit::SessionExited);
                }
            }
        }
    }
}

use crate::client::Attachment;
use crate::relay::client_error_is_retryable;
use crate::relay::control::{ControlChannel, client_error};
use crate::relay::lease::LeaseController;
use crate::relay::output::{EventAction, write_event};
use crate::relay::{RelayCancellation, RelayError, RelayExit, TerminalSize};
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};
use tokio::sync::mpsc;
use uuid::Uuid;
#[path = "attachment_input.rs"]
pub(super) mod input;
use input::{PendingInputState, send_pending_input};
pub(super) enum AttachmentExit {
    Relay(RelayExit),
    OutputGap,
    Disconnected,
}
pub(super) struct AttachmentContext<'a> {
    pub(super) terminal_id: Uuid,
    pub(super) runtime_epoch: Uuid,
    pub(super) lease: &'a mut LeaseController,
    pub(super) control: &'a mut ControlChannel,
    pub(super) input: &'a mut (dyn AsyncRead + Unpin + Send),
    pub(super) output: &'a mut (dyn AsyncWrite + Unpin + Send),
    pub(super) resize_events: &'a mut mpsc::Receiver<TerminalSize>,
    pub(super) receive_resize: &'a mut bool,
    pub(super) current_size: &'a mut Option<TerminalSize>,
    pub(super) input_buffer: &'a mut [u8],
    pub(super) pending_input: &'a mut PendingInputState,
    pub(super) sequence: &'a mut u64,
    pub(super) cancellation: &'a RelayCancellation,
}

pub(super) async fn relay_attachment(
    attachment: &mut Attachment,
    context: AttachmentContext<'_>,
) -> Result<AttachmentExit, RelayError> {
    let AttachmentContext {
        terminal_id,
        runtime_epoch,
        lease,
        control,
        input,
        output,
        resize_events,
        receive_resize,
        current_size,
        input_buffer,
        pending_input,
        sequence,
        cancellation,
    } = context;
    loop {
        let event = {
            let next_event = attachment.next_event();
            tokio::pin!(next_event);
            loop {
                if !lease.is_terminal() && pending_input.has_data() {
                    lease.retain_control_intent();
                }
                if !lease.is_terminal() && pending_input.has_data() && lease.generation().is_some()
                {
                    if !lease.control_ready() {
                        if lease.maintenance_due() {
                            cancellation.wait(lease.maintain()).await??;
                        }
                        cancellation
                            .wait(apply_current_size(control, lease, *current_size))
                            .await??;
                        lease.announce_control_ready();
                        if !lease.control_ready() {
                            continue;
                        }
                    }
                    cancellation
                        .wait(send_pending_input(control, lease, pending_input))
                        .await??;
                    continue;
                }
                if !lease.is_terminal() && pending_input.is_closed() && !pending_input.has_data() {
                    cancellation.wait(output.flush()).await??;
                    let exit = if lease.generation().is_some() {
                        RelayExit::InputClosed
                    } else {
                        RelayExit::ReadOnlyInputClosed
                    };
                    return Ok(AttachmentExit::Relay(exit));
                }
                let maintenance = tokio::time::sleep_until(lease.maintenance_deadline());
                tokio::pin!(maintenance);
                let read_capacity = pending_input.read_capacity(input_buffer.len());
                tokio::select! {
                    _ = cancellation.cancelled() => return Err(RelayError::Cancelled),
                    read = input.read(&mut input_buffer[..read_capacity]),
                        if !lease.is_terminal() && read_capacity > 0 => {
                        let length = read?;
                        if length == 0 {
                            pending_input.close();
                        } else {
                            pending_input.append(&input_buffer[..length]);
                        }
                    }
                    event = &mut next_event => break event,
                    size = resize_events.recv(),
                        if !lease.is_terminal() && *receive_resize => match size {
                        Some(size) if *current_size != Some(size) => {
                            *current_size = Some(size);
                            cancellation
                                .wait(apply_current_size(control, lease, *current_size))
                                .await??;
                            lease.announce_control_ready();
                        }
                        Some(_) => {}
                        None => *receive_resize = false,
                    },
                    _ = &mut maintenance, if !lease.is_terminal() => {
                        cancellation.wait(lease.maintain()).await??;
                        if lease.generation().is_some() && !lease.control_ready() {
                            cancellation
                                .wait(apply_current_size(control, lease, *current_size))
                                .await??;
                        }
                        lease.announce_control_ready();
                    }
                }
            }
        };
        let event = match event {
            Ok(event) => event,
            Err(error)
                if client_error_is_retryable(&error)
                    || crate::relay::host::observation_interrupted(&error) =>
            {
                return Ok(AttachmentExit::Disconnected);
            }
            Err(error) => return Err(error.into()),
        };
        let action = cancellation
            .wait(write_event(
                output,
                &event,
                terminal_id,
                runtime_epoch,
                sequence,
            ))
            .await??;
        lease.observe_event(&event)?;
        match action {
            EventAction::Continue => {}
            EventAction::Reattach => return Ok(AttachmentExit::OutputGap),
            EventAction::SessionExited => {
                cancellation.wait(output.flush()).await??;
                return Ok(AttachmentExit::Relay(RelayExit::SessionExited));
            }
        }
    }
}
pub(super) async fn apply_current_size(
    control: &mut ControlChannel,
    lease: &mut LeaseController,
    size: Option<TerminalSize>,
) -> Result<(), RelayError> {
    loop {
        let Some((generation, writer_capability)) = lease.mutation_credentials() else {
            return Ok(());
        };
        let result = match size {
            Some(size) => control.resize(generation, &writer_capability, size).await,
            None => Ok(()),
        };
        match result {
            Ok(()) if lease.mark_generation_prepared(generation) => return Ok(()),
            Ok(()) => {}
            Err(error) => match client_error(&error) {
                Some(client) if lease.mutation_rejected(client).await? => {
                    if lease.generation().is_none() {
                        return Ok(());
                    }
                }
                _ => return Err(error),
            },
        }
    }
}

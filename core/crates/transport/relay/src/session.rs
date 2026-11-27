use super::constants::INPUT_BUFFER_BYTES;
use super::control::ControlChannel;
use super::lease::LeaseController;
use super::output::write_initial;
use super::{RelayCancellation, RelayError, RelayExit, RelayOptions, RelayRuntime, TerminalSize};
use crate::client::Attachment;
use tokio::io::{AsyncRead, AsyncWrite};
use tokio::sync::mpsc;
#[cfg(all(test, feature = "yolo-unit-tests"))]
use {super::RelayStatus, uuid::Uuid};

#[path = "session/attachment.rs"]
mod attachment;
use attachment::input::PendingInputState;
use attachment::{AttachmentContext, AttachmentExit, apply_current_size, relay_attachment};
#[path = "session/completed.rs"]
mod completed;
#[path = "session/recovery.rs"]
mod recovery;
use completed::relay_completed;
use recovery::{AttachmentRecovery, attach_with_recovery, reconnect_runtime, wait_to_reconnect};
#[path = "session/start.rs"]
mod start;
pub use start::{
    relay_session, relay_session_with_status, relay_session_with_status_and_cancellation,
};

struct RelayIo<'a> {
    input: &'a mut (dyn AsyncRead + Unpin + Send),
    output: &'a mut (dyn AsyncWrite + Unpin + Send),
    resize_events: mpsc::Receiver<TerminalSize>,
}

async fn run_loop(
    runtime: &RelayRuntime,
    options: &RelayOptions,
    lease: &mut LeaseController,
    control: &mut ControlChannel,
    initial_attachment: Attachment,
    initial_sequence: u64,
    io: RelayIo<'_>,
    cancellation: &RelayCancellation,
) -> Result<RelayExit, RelayError> {
    let RelayIo {
        input,
        output,
        mut resize_events,
    } = io;
    cancellation.check()?;
    let mut current_size = options.initial_size;
    cancellation
        .wait(apply_current_size(control, lease, current_size))
        .await??;
    cancellation.check()?;
    let mut sequence = initial_sequence;
    let mut reset_before_snapshot = false;
    let mut receive_resize = true;
    let mut input_buffer = [0_u8; INPUT_BUFFER_BYTES];
    let mut pending_input = PendingInputState::new();
    let expected_runtime_epoch = initial_attachment.runtime_epoch();
    let mut pending_attachment = Some(initial_attachment);

    loop {
        cancellation.check()?;
        let recovery = if lease.is_terminal() {
            AttachmentRecovery::TransportOnly
        } else {
            AttachmentRecovery::SessionAware
        };
        let mut attachment = match pending_attachment.take() {
            Some(attachment) => attachment,
            None => {
                attach_with_recovery(
                    runtime,
                    options.terminal_id,
                    sequence,
                    recovery,
                    cancellation,
                )
                .await?
            }
        };
        let runtime_epoch = attachment.runtime_epoch();
        if expected_runtime_epoch != runtime_epoch {
            return Err(RelayError::RuntimeEpochChanged {
                expected: expected_runtime_epoch,
                actual: runtime_epoch,
            });
        }
        sequence = cancellation
            .wait(write_initial(
                output,
                &attachment.initial,
                sequence,
                reset_before_snapshot,
            ))
            .await??;
        cancellation.check()?;
        if lease.maintenance_due() {
            cancellation.wait(lease.maintain()).await??;
            cancellation
                .wait(apply_current_size(control, lease, current_size))
                .await??;
        }
        lease.announce_control_ready();
        let context = AttachmentContext {
            terminal_id: options.terminal_id,
            runtime_epoch,
            lease,
            control,
            input,
            output,
            resize_events: &mut resize_events,
            receive_resize: &mut receive_resize,
            current_size: &mut current_size,
            input_buffer: &mut input_buffer,
            pending_input: &mut pending_input,
            sequence: &mut sequence,
            cancellation,
        };
        let reattach = relay_attachment(&mut attachment, context).await?;
        match reattach {
            AttachmentExit::Relay(exit) => return Ok(exit),
            AttachmentExit::OutputGap => reset_before_snapshot = true,
            AttachmentExit::Disconnected => {
                cancellation.check()?;
                if lease.is_terminal() {
                    reconnect_runtime(
                        runtime,
                        options.terminal_id,
                        AttachmentRecovery::TransportOnly,
                        cancellation,
                    )
                    .await?;
                } else {
                    cancellation.wait(lease.recover_transport()).await??;
                }
                wait_to_reconnect(cancellation).await?;
                reset_before_snapshot = true;
            }
        }
    }
}

#[cfg(test)]
#[cfg(any(not(test), feature = "yolo-unit-tests"))]
include!("../unit_tests/session_test_modules.rs");

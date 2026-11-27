use super::{RelayIo, run_loop};
use crate::relay::control::ControlChannel;
use crate::relay::lease::LeaseController;
use crate::relay::status::RelayStatusSink;
use crate::relay::{
    RelayCancellation, RelayError, RelayExit, RelayOptions, RelayRuntime, RelayStatus, TerminalSize,
};
use crate::worker::{FORCE_SNAPSHOT_AFTER_SEQUENCE, RuntimeRef};
use tokio::io::{AsyncRead, AsyncWrite};
use tokio::sync::mpsc;

pub async fn relay_session<C, R, W>(
    runtime: C,
    options: RelayOptions,
    input: R,
    output: W,
    resize_events: mpsc::Receiver<TerminalSize>,
) -> Result<RelayExit, RelayError>
where
    C: Into<RelayRuntime>,
    R: AsyncRead + Unpin + Send,
    W: AsyncWrite + Unpin + Send,
{
    let (status, _events) = mpsc::unbounded_channel();
    relay_session_core(
        runtime,
        options,
        input,
        output,
        resize_events,
        status.into(),
        RelayCancellation::default(),
    )
    .await
}

pub async fn relay_session_with_status<C, R, W>(
    runtime: C,
    options: RelayOptions,
    input: R,
    output: W,
    resize_events: mpsc::Receiver<TerminalSize>,
    status: mpsc::UnboundedSender<RelayStatus>,
) -> Result<RelayExit, RelayError>
where
    C: Into<RelayRuntime>,
    R: AsyncRead + Unpin + Send,
    W: AsyncWrite + Unpin + Send,
{
    relay_session_core(
        runtime,
        options,
        input,
        output,
        resize_events,
        status.into(),
        RelayCancellation::default(),
    )
    .await
}

pub async fn relay_session_with_status_and_cancellation<C, R, W>(
    runtime: C,
    options: RelayOptions,
    input: R,
    output: W,
    resize_events: mpsc::Receiver<TerminalSize>,
    status: mpsc::UnboundedSender<RelayStatus>,
    cancellation: RelayCancellation,
) -> Result<RelayExit, RelayError>
where
    C: Into<RelayRuntime>,
    R: AsyncRead + Unpin + Send,
    W: AsyncWrite + Unpin + Send,
{
    relay_session_core(
        runtime,
        options,
        input,
        output,
        resize_events,
        status.into(),
        cancellation,
    )
    .await
}

async fn relay_session_core<C, R, W>(
    runtime: C,
    options: RelayOptions,
    mut input: R,
    mut output: W,
    resize_events: mpsc::Receiver<TerminalSize>,
    status: RelayStatusSink,
    cancellation: RelayCancellation,
) -> Result<RelayExit, RelayError>
where
    C: Into<RelayRuntime>,
    R: AsyncRead + Unpin + Send,
    W: AsyncWrite + Unpin + Send,
{
    cancellation.check()?;
    let runtime = runtime.into();
    let prepared = cancellation
        .wait(runtime.prepare(options.terminal_id, &status))
        .await??;
    cancellation.check()?;
    let was_resumed = prepared.was_resumed();
    if let Some((runtime_epoch, expected_exit)) = prepared.into_completed() {
        let result = super::relay_completed(
            &runtime,
            &options,
            &mut output,
            runtime_epoch,
            expected_exit,
            &status,
            &cancellation,
        )
        .await;
        if matches!(&result, Ok(RelayExit::SessionExited)) {
            status.send(RelayStatus::SessionExited);
        }
        return result;
    }
    let attachment_sequence = if was_resumed {
        FORCE_SNAPSHOT_AFTER_SEQUENCE
    } else {
        options.after_sequence
    };
    let attachment = cancellation
        .wait(super::attach_with_recovery(
            &runtime,
            options.terminal_id,
            attachment_sequence,
            super::AttachmentRecovery::SessionAware,
            &cancellation,
        ))
        .await??;
    let target = RuntimeRef::new(options.terminal_id, attachment.runtime_epoch());
    let mut lease = LeaseController::pending(runtime.clone(), &options, target, status.clone());
    let result = async {
        let mut control = cancellation
            .wait(ControlChannel::connect(runtime.clone(), target))
            .await??;
        cancellation.check()?;
        lease.start(options.take_control, &cancellation).await?;
        cancellation.check()?;
        run_loop(
            &runtime,
            &options,
            &mut lease,
            &mut control,
            attachment,
            attachment_sequence,
            RelayIo {
                input: &mut input,
                output: &mut output,
                resize_events,
            },
            &cancellation,
        )
        .await
    }
    .await;
    if matches!(&result, Ok(RelayExit::SessionExited)) {
        status.send(RelayStatus::SessionExited);
    }
    lease.release().await;
    result
}

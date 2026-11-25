use crate::client::Attachment;
use crate::model::ExitRecord;
use crate::relay::constants::CONTROL_RECONNECT_DELAY;
use crate::relay::{
    RelayCancellation, RelayError, RelayRuntime, client_error_is_retryable,
    relay_error_is_retryable,
};
use uuid::Uuid;

#[derive(Clone, Copy)]
pub(super) enum AttachmentRecovery<'a> {
    SessionAware,
    TransportOnly,
    Completed {
        runtime_epoch: Uuid,
        expected_exit: &'a ExitRecord,
    },
}

pub(super) async fn attach_with_recovery(
    runtime: &RelayRuntime,
    terminal_id: Uuid,
    sequence: u64,
    mode: AttachmentRecovery<'_>,
    cancellation: &RelayCancellation,
) -> Result<Attachment, RelayError> {
    loop {
        cancellation.check()?;
        let attempt = cancellation
            .wait(runtime.client().attach(terminal_id, sequence))
            .await?;
        match attempt {
            Ok(attachment) => return Ok(attachment),
            Err(error)
                if client_error_is_retryable(&error)
                    || crate::relay::host::observation_interrupted(&error) =>
            {
                reconnect_runtime(runtime, terminal_id, mode, cancellation).await?;
                if matches!(mode, AttachmentRecovery::TransportOnly) {
                    if let Err(error) = cancellation
                        .wait(runtime.reject_user_terminated(terminal_id))
                        .await?
                    {
                        if !relay_error_is_retryable(&error) {
                            return Err(error);
                        }
                    }
                }
                wait_to_reconnect(cancellation).await?;
            }
            Err(error) => return Err(error.into()),
        }
    }
}

pub(super) async fn reconnect_runtime(
    runtime: &RelayRuntime,
    terminal_id: Uuid,
    mode: AttachmentRecovery<'_>,
    cancellation: &RelayCancellation,
) -> Result<(), RelayError> {
    loop {
        let recovery = async {
            match mode {
                AttachmentRecovery::SessionAware => runtime.reconnect(terminal_id).await,
                AttachmentRecovery::TransportOnly => runtime.reconnect_transport().await,
                AttachmentRecovery::Completed {
                    runtime_epoch,
                    expected_exit,
                } => {
                    runtime
                        .reconnect_completed(terminal_id, runtime_epoch, expected_exit)
                        .await
                }
            }
        };
        let result = cancellation.wait(recovery).await?;
        match result {
            Ok(()) => return Ok(()),
            Err(error) if relay_error_is_retryable(&error) => {
                cancellation
                    .wait(tokio::time::sleep(CONTROL_RECONNECT_DELAY))
                    .await?;
            }
            Err(error) => return Err(error),
        }
    }
}

pub(super) async fn wait_to_reconnect(cancellation: &RelayCancellation) -> Result<(), RelayError> {
    cancellation
        .wait(tokio::time::sleep(CONTROL_RECONNECT_DELAY))
        .await?;
    Ok(())
}

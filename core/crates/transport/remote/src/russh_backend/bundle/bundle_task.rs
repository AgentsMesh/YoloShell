use super::{
    HostKeyHandler, bundle_cancel::Cancellation, bundle_raw::*,
    managed_raw_sftp_session::ManagedRawSftpSession,
};
use crate::remote::{
    BundleRetentionReason, BundleStageRequest, BundleStageResult, RemoteCleanupReservation,
    RemoteError, cleanup_registry::StageRetention,
};
use std::{sync::Arc, time::Duration};
use tokio::{sync::oneshot, task::JoinHandle};

const TASK_ENDED: &str = "bundle staging task ended before delivery";
const TASK_CLEANUP: &str = "bundle staging channel cancellation did not settle";

pub(super) struct StageTask {
    ready: oneshot::Receiver<Result<BundleStageResult, RemoteError>>,
    adopt: Option<oneshot::Sender<()>>,
    cancelled: Cancellation,
    task: Option<JoinHandle<Result<(), RemoteError>>>,
    connection: Arc<russh::client::Handle<HostKeyHandler>>,
    reservation: Option<RemoteCleanupReservation>,
    limit: Duration,
}

impl StageTask {
    pub fn start(
        raw: ManagedRawSftpSession,
        connection: Arc<russh::client::Handle<HostKeyHandler>>,
        request: BundleStageRequest,
        uid: u32,
        limit: Duration,
        reservation: RemoteCleanupReservation,
        retention: StageRetention,
    ) -> Self {
        let (send, ready) = oneshot::channel();
        let (adopt, accepted) = oneshot::channel();
        let cancelled = Cancellation::default();
        let cancellation = cancelled.clone();
        let task = tokio::spawn(async move {
            // Cancelling the operation interrupts every pending metadata/write/read wait.
            let result = cancellation
                .run(super::bundle_stage::stage(
                    &raw, &request, uid, &retention, limit,
                ))
                .await;
            let closed = raw.close(SCOPE).await;
            let channel_error = closed.as_ref().err().map(ToString::to_string);
            let result = super::deployment_operation::settle(result, closed);
            match result {
                Ok(value) => {
                    if send.send(Ok(value)).is_ok() && accepted.await.is_ok() {
                        retention.accepted();
                    } else {
                        retention.retain(BundleRetentionReason::Undelivered);
                    }
                }
                Err(cause) => {
                    let reason = if cancellation.is_cancelled() {
                        BundleRetentionReason::Cancelled
                    } else {
                        BundleRetentionReason::Failed
                    };
                    let error = match retention.retain(reason) {
                        Some(stage) => RemoteError::BundleStageRetained {
                            stage,
                            cause: Box::new(cause),
                        },
                        None => cause,
                    };
                    let _ = send.send(Err(error));
                }
            }
            // Retained filesystem evidence is distinct from settled channel resources.
            channel_error.map_or(Ok(()), |error| Err(RemoteError::transport(error)))
        });
        Self {
            ready,
            adopt: Some(adopt),
            cancelled,
            task: Some(task),
            connection,
            reservation: Some(reservation),
            limit,
        }
    }

    pub async fn finish(mut self) -> Result<BundleStageResult, RemoteError> {
        let result = (&mut self.ready)
            .await
            .map_err(|_| RemoteError::transport(TASK_ENDED))??;
        self.adopt
            .take()
            .unwrap()
            .send(())
            .map_err(|_| RemoteError::transport(TASK_ENDED))?;
        self.track_completion();
        Ok(result)
    }

    fn track_completion(&mut self) {
        if let (Some(task), Some(reservation)) = (self.task.take(), self.reservation.take()) {
            let connection = self.connection.clone();
            let limit = self.limit;
            reservation.launch(super::channel_open_cleanup::settle_cleanup_task(
                connection,
                task,
                limit,
                TASK_CLEANUP,
            ));
        }
    }
}

impl Drop for StageTask {
    fn drop(&mut self) {
        self.cancelled.cancel();
        self.adopt.take();
        self.track_completion();
    }
}

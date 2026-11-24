use super::approved_channel::ApprovalBuffer;
use super::channel_open_custody::SessionChannel;
use super::channel_open_disconnect::{
    disconnect_and_abort, disconnect_retained, settle_escalation, settle_primary,
};
use super::sftp_channel_driver;
use super::{HostKeyHandler, RemoteError, ssh_error};
use crate::remote::constants::{
    CHANNEL_CLOSE_CLEANUP_TIMEOUT_MESSAGE, SFTP_CHANNEL_ABORT_SETTLEMENT_MESSAGE,
    SFTP_CHANNEL_DRIVER_TASK_FAILURE_CONTEXT, SSH_CLEANUP_ABORT_SETTLEMENT_TIMEOUT,
};
use crate::remote::{RemoteCleanupReservation, RemoteError as Error};
use russh::client::Handle;
use std::sync::Arc;
use std::time::Duration;
use tokio::io::DuplexStream;
use tokio::sync::oneshot;
use tokio::task::JoinHandle;

type DriverTask = JoinHandle<Result<(), RemoteError>>;

pub(super) struct TransferredChannelCustody {
    session: Option<Arc<Handle<HostKeyHandler>>>,
    close: Option<oneshot::Sender<()>>,
    task: Option<DriverTask>,
    reservation: Option<RemoteCleanupReservation>,
    cleanup_timeout: Duration,
    disconnect_required: bool,
}

impl TransferredChannelCustody {
    #[cfg(test)]
    pub(super) fn from_driver_task_for_test(
        session: Arc<Handle<HostKeyHandler>>,
        reservation: RemoteCleanupReservation,
        cleanup_timeout: Duration,
        task: DriverTask,
    ) -> Self {
        let (close, receiver) = oneshot::channel();
        drop(receiver);
        Self {
            session: Some(session),
            close: Some(close),
            task: Some(task),
            reservation: Some(reservation),
            cleanup_timeout,
            disconnect_required: false,
        }
    }

    pub(super) async fn close(mut self) -> Result<(), RemoteError> {
        self.request_close();
        let result = tokio::time::timeout(self.cleanup_timeout, self.task.as_mut().unwrap()).await;
        match result {
            Ok(Ok(Ok(()))) => {
                self.complete();
                Ok(())
            }
            Ok(Ok(Err(primary))) => self.disconnect_after_completed(primary).await,
            Ok(Err(error)) => {
                self.disconnect_after_completed(ssh_error(format!(
                    "{SFTP_CHANNEL_DRIVER_TASK_FAILURE_CONTEXT}: {error}"
                )))
                .await
            }
            Err(_) => self.disconnect_after_timeout().await,
        }
    }

    pub(super) fn fail<T>(mut self, primary: RemoteError) -> Result<T, RemoteError> {
        self.disconnect_required = true;
        self.launch();
        Err(primary)
    }

    pub(super) fn fail_task<T, Output>(
        mut self,
        primary: RemoteError,
        task: JoinHandle<Output>,
    ) -> Result<T, RemoteError>
    where
        Output: Send + 'static,
    {
        self.launch_with_task(task);
        Err(primary)
    }

    pub(super) fn abandon_task<Output>(mut self, task: JoinHandle<Output>)
    where
        Output: Send + 'static,
    {
        self.launch_with_task(task);
    }

    fn request_close(&mut self) {
        if let Some(close) = self.close.take() {
            let _ = close.send(());
        }
    }

    async fn disconnect_after_completed(
        &mut self,
        primary: RemoteError,
    ) -> Result<(), RemoteError> {
        self.task.take();
        self.disconnect_required = true;
        let settlement =
            disconnect_retained(self.session.as_ref().unwrap(), self.cleanup_timeout).await;
        self.complete();
        settle_primary(primary, settlement)
    }

    async fn disconnect_after_timeout(&mut self) -> Result<(), RemoteError> {
        self.disconnect_required = true;
        let primary = Error::transport(CHANNEL_CLOSE_CLEANUP_TIMEOUT_MESSAGE);
        let disconnected =
            disconnect_retained(self.session.as_ref().unwrap(), self.cleanup_timeout).await;
        let aborted = abort_task(self.task.take().unwrap()).await;
        self.complete();
        settle_primary(primary, combine(disconnected, aborted))
    }

    fn complete(&mut self) {
        self.close.take();
        self.task.take();
        self.session.take();
        self.reservation.take();
        self.disconnect_required = false;
    }

    fn launch_with_task<Output>(&mut self, task: JoinHandle<Output>)
    where
        Output: Send + 'static,
    {
        let session = self.session.take().unwrap();
        let driver = self.task.take().unwrap();
        let reservation = self.reservation.take().unwrap();
        let timeout = self.cleanup_timeout;
        self.close.take();
        self.disconnect_required = false;
        reservation.launch(disconnect_and_abort_pair(session, driver, task, timeout));
    }

    fn launch(&mut self) {
        let Some(session) = self.session.take() else {
            return;
        };
        let task = self.task.take();
        let Some(reservation) = self.reservation.take() else {
            return;
        };
        let timeout = self.cleanup_timeout;
        let close = self.close.take();
        let disconnect = self.disconnect_required;
        self.disconnect_required = false;
        reservation.launch(async move {
            if disconnect {
                disconnect_and_abort(session, task, timeout).await
            } else {
                close_driver(session, close, task.unwrap(), timeout).await
            }
        });
    }
}

impl Drop for TransferredChannelCustody {
    fn drop(&mut self) {
        self.launch();
    }
}

include!("channel_transfer_cleanup.rs");
include!("channel_transfer_start.rs");

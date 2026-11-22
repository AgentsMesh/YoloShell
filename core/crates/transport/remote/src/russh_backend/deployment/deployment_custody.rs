use super::deployment_fs::cleanup_temporary;
use super::sftp::open_sftp_session_for_cleanup;
use super::{HostKeyHandler, RusshConnection};
use russh::client;
use std::sync::Arc;
use std::time::Duration;

use crate::remote::constants::TEMPORARY_PATH_CLEANUP_TIMEOUT_MESSAGE;
use crate::remote::{RemoteCleanupReservation, RemoteError};

pub(super) struct TemporaryPathCustody {
    connection: Option<Arc<client::Handle<HostKeyHandler>>>,
    path: String,
    cleanup_timeout: Duration,
    reservation: Option<RemoteCleanupReservation>,
    channel_reservation: Option<RemoteCleanupReservation>,
}

impl TemporaryPathCustody {
    pub(super) fn new(
        connection: Arc<client::Handle<HostKeyHandler>>,
        path: impl Into<String>,
        cleanup_timeout: Duration,
        reservation: RemoteCleanupReservation,
        channel_reservation: RemoteCleanupReservation,
    ) -> Self {
        Self {
            connection: Some(connection),
            path: path.into(),
            cleanup_timeout,
            reservation: Some(reservation),
            channel_reservation: Some(channel_reservation),
        }
    }

    pub(super) fn release(mut self) {
        self.connection.take();
        self.reservation.take();
        self.channel_reservation.take();
    }
}

impl RusshConnection {
    pub(super) fn temporary_path_custody(
        &self,
        path: &str,
    ) -> Result<TemporaryPathCustody, RemoteError> {
        let reservation = self.cleanup.reserve()?;
        let channel_reservation = self.cleanup.reserve()?;
        Ok(TemporaryPathCustody::new(
            Arc::clone(&self.session),
            path,
            self.operation_timeout,
            reservation,
            channel_reservation,
        ))
    }
}

impl Drop for TemporaryPathCustody {
    fn drop(&mut self) {
        let Some(connection) = self.connection.take() else {
            return;
        };
        let path = std::mem::take(&mut self.path);
        let cleanup_timeout = self.cleanup_timeout;
        let Some(reservation) = self.reservation.take() else {
            return;
        };
        let Some(channel_reservation) = self.channel_reservation.take() else {
            return;
        };
        reservation.launch(async move {
            let cleanup_connection = Arc::clone(&connection);
            let cleanup = tokio::spawn(async move {
                let session = open_sftp_session_for_cleanup(
                    cleanup_connection,
                    cleanup_timeout,
                    channel_reservation,
                )
                .await?;
                let removed = cleanup_temporary(&session, &path, cleanup_timeout).await;
                let closed = session.close().await;
                super::deployment_operation::settle(removed, closed)
            });
            super::channel_open_cleanup::settle_cleanup_task(
                connection,
                cleanup,
                cleanup_timeout,
                TEMPORARY_PATH_CLEANUP_TIMEOUT_MESSAGE,
            )
            .await
        });
    }
}

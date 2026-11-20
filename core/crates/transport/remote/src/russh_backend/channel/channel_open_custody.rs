use super::channel_open_cleanup::{cleanup_opening_task, close_session_channel};
use super::{HostKeyHandler, RusshConnection, ssh_error};
use crate::remote::HubPeerEndpoint;
use crate::remote::constants::{CHANNEL_OPEN_TASK_FAILURE_CONTEXT, OPERATION_TIMEOUT_MESSAGE};
use crate::remote::{RemoteCleanupReservation, RemoteError};
use russh::Channel;
use russh::client::{Handle, Msg};
use std::sync::Arc;
use std::time::Duration;
use tokio::task::JoinHandle;
use tokio::time::{Instant, timeout_at};

pub(super) type SessionChannel = Channel<Msg>;
pub(super) type SessionChannelOpenTask = JoinHandle<Result<SessionChannel, RemoteError>>;

pub(super) struct OpenedSessionChannel {
    session: Option<Arc<Handle<HostKeyHandler>>>,
    channel: Option<SessionChannel>,
    reservation: Option<RemoteCleanupReservation>,
    cleanup_timeout: Duration,
}

struct OpeningSessionChannel {
    session: Arc<Handle<HostKeyHandler>>,
    task: Option<SessionChannelOpenTask>,
    reservation: Option<RemoteCleanupReservation>,
    cleanup_timeout: Duration,
    disconnect_required: bool,
}

impl RusshConnection {
    pub(super) async fn open_session_channel(&self) -> Result<OpenedSessionChannel, RemoteError> {
        let deadline = Instant::now() + self.operation_timeout;
        self.open_session_channel_until(deadline).await
    }

    pub(super) async fn open_session_channel_until(
        &self,
        deadline: Instant,
    ) -> Result<OpenedSessionChannel, RemoteError> {
        let reservation = self.cleanup.reserve()?;
        OpeningSessionChannel::new(
            Arc::clone(&self.session),
            reservation,
            self.operation_timeout,
            None,
        )
        .wait_until(deadline)
        .await
    }

    pub(super) async fn open_hub_peer_channel_until(
        &self,
        endpoint: HubPeerEndpoint,
        deadline: Instant,
    ) -> Result<OpenedSessionChannel, RemoteError> {
        let reservation = self.cleanup.reserve()?;
        OpeningSessionChannel::new(
            Arc::clone(&self.session),
            reservation,
            self.operation_timeout,
            Some(endpoint.into_path()),
        )
        .wait_until(deadline)
        .await
    }
}

impl OpeningSessionChannel {
    fn new(
        session: Arc<Handle<HostKeyHandler>>,
        reservation: RemoteCleanupReservation,
        cleanup_timeout: Duration,
        endpoint: Option<std::path::PathBuf>,
    ) -> Self {
        let opening_session = Arc::clone(&session);
        let task = tokio::spawn(async move {
            match endpoint {
                Some(path) => opening_session
                    .channel_open_direct_streamlocal(path.to_string_lossy())
                    .await
                    .map_err(super::hub_peer::open_error),
                None => opening_session
                    .channel_open_session()
                    .await
                    .map_err(ssh_error),
            }
        });
        Self {
            session,
            task: Some(task),
            reservation: Some(reservation),
            cleanup_timeout,
            disconnect_required: false,
        }
    }

    async fn wait_until(mut self, deadline: Instant) -> Result<OpenedSessionChannel, RemoteError> {
        let result = timeout_at(deadline, self.task.as_mut().unwrap()).await;
        match result {
            Ok(Ok(Ok(channel))) => {
                self.task.take();
                Ok(OpenedSessionChannel {
                    session: Some(Arc::clone(&self.session)),
                    channel: Some(channel),
                    reservation: self.reservation.take(),
                    cleanup_timeout: self.cleanup_timeout,
                })
            }
            Ok(Ok(Err(error))) => {
                self.task.take();
                Err(error)
            }
            Ok(Err(error)) => {
                self.task.take();
                self.disconnect_required = true;
                Err(RemoteError::transport(format!(
                    "{CHANNEL_OPEN_TASK_FAILURE_CONTEXT}: {error}"
                )))
            }
            Err(_) => Err(RemoteError::transport(OPERATION_TIMEOUT_MESSAGE)),
        }
    }
}

impl Drop for OpeningSessionChannel {
    fn drop(&mut self) {
        if self.task.is_none() && !self.disconnect_required {
            return;
        }
        let Some(reservation) = self.reservation.take() else {
            return;
        };
        let session = Arc::clone(&self.session);
        let task = self.task.take();
        let timeout = self.cleanup_timeout;
        reservation.launch(cleanup_opening_task(
            session,
            task,
            timeout,
            self.disconnect_required,
        ));
    }
}

impl OpenedSessionChannel {
    pub(super) fn channel(&mut self) -> &mut SessionChannel {
        self.channel.as_mut().unwrap()
    }

    pub(super) fn into_parts(
        mut self,
    ) -> (
        Arc<Handle<HostKeyHandler>>,
        SessionChannel,
        RemoteCleanupReservation,
    ) {
        (
            self.session.take().unwrap(),
            self.channel.take().unwrap(),
            self.reservation.take().unwrap(),
        )
    }

    pub(super) fn release(mut self) -> SessionChannel {
        let channel = self.channel.take().unwrap();
        self.session.take();
        self.reservation.take();
        channel
    }
}

impl Drop for OpenedSessionChannel {
    fn drop(&mut self) {
        let Some(channel) = self.channel.take() else {
            return;
        };
        let Some(session) = self.session.take() else {
            return;
        };
        let Some(reservation) = self.reservation.take() else {
            return;
        };
        reservation.launch(close_session_channel(
            session,
            channel,
            self.cleanup_timeout,
        ));
    }
}

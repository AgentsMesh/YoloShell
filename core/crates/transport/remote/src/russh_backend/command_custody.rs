use russh::Channel;
use russh::client::{Handle, Msg};
use std::sync::Arc;
use std::time::Duration;

use crate::remote::constants::COMMAND_KILL_CLEANUP_TIMEOUT_MESSAGE;
use crate::remote::{RemoteCleanupReservation, RemoteError};

pub(super) struct CommandChannelCustody {
    session: Arc<Handle<super::HostKeyHandler>>,
    channel: Option<Channel<Msg>>,
    cleanup_timeout: Duration,
    reservation: Option<RemoteCleanupReservation>,
}

impl CommandChannelCustody {
    pub(super) fn new(
        session: Arc<Handle<super::HostKeyHandler>>,
        channel: Channel<Msg>,
        cleanup_timeout: Duration,
        reservation: RemoteCleanupReservation,
    ) -> Self {
        Self {
            session,
            channel: Some(channel),
            cleanup_timeout,
            reservation: Some(reservation),
        }
    }

    pub(super) fn channel(&mut self) -> &mut Channel<Msg> {
        let Some(channel) = self.channel.as_mut() else {
            unreachable!()
        };
        channel
    }

    pub(super) fn release(mut self) {
        self.channel.take();
        self.reservation.take();
    }
}

impl Drop for CommandChannelCustody {
    fn drop(&mut self) {
        let Some(channel) = self.channel.take() else {
            return;
        };
        let Some(reservation) = self.reservation.take() else {
            return;
        };
        let cleanup_timeout = self.cleanup_timeout;
        reservation.launch(cleanup_channel(
            Arc::clone(&self.session),
            channel,
            cleanup_timeout,
        ));
    }
}

async fn cleanup_channel(
    session: Arc<Handle<super::HostKeyHandler>>,
    channel: Channel<Msg>,
    cleanup_timeout: Duration,
) -> Result<(), RemoteError> {
    let killed = match tokio::time::timeout(cleanup_timeout, channel.signal(russh::Sig::KILL)).await
    {
        Ok(result) => result.map_err(super::ssh_error),
        Err(_) => Err(RemoteError::transport(COMMAND_KILL_CLEANUP_TIMEOUT_MESSAGE)),
    };
    let closed =
        super::channel_open_cleanup::close_session_channel(session, channel, cleanup_timeout).await;
    match (killed, closed) {
        (Ok(()), result) => result,
        (Err(primary), Ok(())) => Err(primary),
        (Err(primary), Err(settlement)) => Err(RemoteError::LifecycleSettlement {
            primary: Box::new(primary),
            settlement: Box::new(settlement),
        }),
    }
}

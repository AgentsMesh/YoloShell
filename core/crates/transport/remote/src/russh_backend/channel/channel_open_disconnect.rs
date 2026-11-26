use super::{HostKeyHandler, ssh_error};
use crate::remote::RemoteError;
use crate::remote::constants::{
    CHANNEL_OPEN_ABORT_SETTLEMENT_MESSAGE, SSH_CLEANUP_ABORT_SETTLEMENT_TIMEOUT,
    SSH_DISCONNECT_CLEANUP_TIMEOUT_MESSAGE, SSH_DISCONNECT_DESCRIPTION,
    SSH_DISCONNECT_LANGUAGE_TAG,
};
use russh::client::Handle;
use std::sync::Arc;
use std::time::Duration;

pub(super) async fn disconnect_session(
    session: &Handle<HostKeyHandler>,
    timeout: Duration,
) -> Result<(), RemoteError> {
    if session.is_closed() {
        return Ok(());
    }
    tokio::time::timeout(timeout, async {
        send_disconnect(session).await?;
        wait_until_closed(session).await;
        Ok(())
    })
    .await
    .map_err(|_| RemoteError::transport(SSH_DISCONNECT_CLEANUP_TIMEOUT_MESSAGE))?
}

pub(super) async fn disconnect_retained(
    session: &Handle<HostKeyHandler>,
    timeout: Duration,
) -> Result<(), RemoteError> {
    let Err(primary) = disconnect_session(session, timeout).await else {
        return Ok(());
    };
    if session.is_closed() {
        return Ok(());
    }
    loop {
        match send_disconnect(session).await {
            Ok(()) => break,
            Err(_) if session.is_closed() => return Err(primary),
            Err(_) => tokio::task::yield_now().await,
        }
    }
    wait_until_closed(session).await;
    Err(primary)
}

pub(super) async fn disconnect_and_abort<T>(
    session: Arc<Handle<HostKeyHandler>>,
    task: Option<tokio::task::JoinHandle<T>>,
    timeout: Duration,
) -> Result<(), RemoteError>
where
    T: Send + 'static,
{
    let disconnected = disconnect_retained(&session, timeout).await;
    let Some(task) = task else {
        return disconnected;
    };
    task.abort();
    let aborted = tokio::time::timeout(SSH_CLEANUP_ABORT_SETTLEMENT_TIMEOUT, task)
        .await
        .map(|_| ())
        .map_err(|_| RemoteError::transport(CHANNEL_OPEN_ABORT_SETTLEMENT_MESSAGE));
    settle_pair(disconnected, aborted)
}

pub(super) fn settle_escalation(
    primary: RemoteError,
    escalation: Result<(), RemoteError>,
) -> Result<(), RemoteError> {
    match escalation {
        Ok(()) => Ok(()),
        Err(settlement) => Err(RemoteError::LifecycleSettlement {
            primary: Box::new(primary),
            settlement: Box::new(settlement),
        }),
    }
}

pub(super) fn settle_primary(
    primary: RemoteError,
    settlement: Result<(), RemoteError>,
) -> Result<(), RemoteError> {
    match settlement {
        Ok(()) => Err(primary),
        Err(settlement) => Err(RemoteError::LifecycleSettlement {
            primary: Box::new(primary),
            settlement: Box::new(settlement),
        }),
    }
}

fn settle_pair(
    primary: Result<(), RemoteError>,
    settlement: Result<(), RemoteError>,
) -> Result<(), RemoteError> {
    match (primary, settlement) {
        (Ok(()), Ok(())) => Ok(()),
        (Err(primary), Ok(())) => Err(primary),
        (Ok(()), Err(settlement)) => Err(settlement),
        (Err(primary), Err(settlement)) => Err(RemoteError::LifecycleSettlement {
            primary: Box::new(primary),
            settlement: Box::new(settlement),
        }),
    }
}

async fn send_disconnect(session: &Handle<HostKeyHandler>) -> Result<(), RemoteError> {
    match session
        .disconnect(
            russh::Disconnect::ByApplication,
            SSH_DISCONNECT_DESCRIPTION,
            SSH_DISCONNECT_LANGUAGE_TAG,
        )
        .await
    {
        Ok(()) => Ok(()),
        Err(_) if session.is_closed() => Ok(()),
        Err(error) => Err(ssh_error(error)),
    }
}

async fn wait_until_closed(session: &Handle<HostKeyHandler>) {
    while !session.is_closed() {
        tokio::task::yield_now().await;
    }
}

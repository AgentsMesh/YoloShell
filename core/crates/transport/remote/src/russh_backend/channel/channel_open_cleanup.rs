use super::channel_open_custody::{SessionChannel, SessionChannelOpenTask};
use super::channel_open_disconnect::{
    disconnect_and_abort, disconnect_retained, settle_escalation, settle_primary,
};
use super::{HostKeyHandler, ssh_error};
use crate::remote::RemoteError;
use crate::remote::constants::{
    CHANNEL_CLOSE_CLEANUP_TIMEOUT_MESSAGE, CHANNEL_OPEN_TASK_FAILURE_CONTEXT,
};
use russh::ChannelMsg;
use russh::client::Handle;
use std::sync::Arc;
use std::time::Duration;

pub(super) async fn cleanup_opening_task(
    session: Arc<Handle<HostKeyHandler>>,
    mut task: Option<SessionChannelOpenTask>,
    timeout: Duration,
    disconnect_required: bool,
) -> Result<(), RemoteError> {
    if disconnect_required {
        return disconnect_and_abort(session, task, timeout).await;
    }
    let Some(mut opening) = task.take() else {
        return Ok(());
    };
    match tokio::time::timeout(timeout, &mut opening).await {
        Ok(Ok(Ok(channel))) => close_session_channel(session, channel, timeout).await,
        Ok(Ok(Err(_))) => Ok(()),
        Ok(Err(error)) => {
            let primary =
                RemoteError::transport(format!("{CHANNEL_OPEN_TASK_FAILURE_CONTEXT}: {error}"));
            settle_escalation(primary, disconnect_retained(&session, timeout).await)
        }
        Err(_) => disconnect_and_abort(session, Some(opening), timeout).await,
    }
}

pub(super) async fn close_session_channel(
    session: Arc<Handle<HostKeyHandler>>,
    mut channel: SessionChannel,
    timeout: Duration,
) -> Result<(), RemoteError> {
    let closed = match tokio::time::timeout(timeout, async {
        channel.close().await.map_err(ssh_error)?;
        while let Some(message) = channel.wait().await {
            if matches!(message, ChannelMsg::Close) {
                return Ok(());
            }
        }
        Ok(())
    })
    .await
    {
        Ok(result) => result,
        Err(_) => Err(RemoteError::transport(
            CHANNEL_CLOSE_CLEANUP_TIMEOUT_MESSAGE,
        )),
    };
    match closed {
        Ok(()) => Ok(()),
        Err(primary) => settle_escalation(primary, disconnect_retained(&session, timeout).await),
    }
}

pub(super) async fn settle_cleanup_task(
    session: Arc<Handle<HostKeyHandler>>,
    mut task: tokio::task::JoinHandle<Result<(), RemoteError>>,
    timeout: Duration,
    timeout_message: &'static str,
) -> Result<(), RemoteError> {
    match tokio::time::timeout(timeout, &mut task).await {
        Ok(Ok(result)) => result,
        Ok(Err(error)) => {
            let primary =
                RemoteError::transport(format!("{CHANNEL_OPEN_TASK_FAILURE_CONTEXT}: {error}"));
            settle_primary(primary, disconnect_retained(&session, timeout).await)
        }
        Err(_) => {
            let primary = RemoteError::transport(timeout_message);
            let escalation = disconnect_and_abort(session, Some(task), timeout).await;
            settle_primary(primary, escalation)
        }
    }
}

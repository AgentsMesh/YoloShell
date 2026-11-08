use super::super::constants::{CHANNEL_CLOSED_MESSAGE, UNEXPECTED_CHANNEL_REPLY_MESSAGE};
use super::approved_channel::ApprovalBuffer;
use super::*;
use russh::ChannelMsg;

pub(super) async fn exec(
    channel: &mut russh::Channel<russh::client::Msg>,
    command: String,
) -> Result<ApprovalBuffer, RemoteError> {
    channel.exec(true, command).await.map_err(ssh_error)?;
    approval(channel).await
}

pub(super) async fn subsystem(
    channel: &mut russh::Channel<russh::client::Msg>,
    name: &str,
) -> Result<ApprovalBuffer, RemoteError> {
    channel
        .request_subsystem(true, name)
        .await
        .map_err(ssh_error)?;
    approval(channel).await
}

async fn approval(
    channel: &mut russh::Channel<russh::client::Msg>,
) -> Result<ApprovalBuffer, RemoteError> {
    let mut buffered = ApprovalBuffer::new();
    loop {
        match channel.wait().await {
            Some(ChannelMsg::Success) => return Ok(buffered),
            Some(ChannelMsg::Failure) => return Err(RemoteError::ChannelRejected),
            Some(message @ ChannelMsg::Data { .. })
            | Some(message @ ChannelMsg::ExtendedData { .. })
            | Some(message @ ChannelMsg::ExitStatus { .. })
            | Some(message @ ChannelMsg::ExitSignal { .. })
            | Some(message @ ChannelMsg::Eof)
            | Some(message @ ChannelMsg::Close) => buffered.push(message)?,
            Some(ChannelMsg::WindowAdjusted { .. }) => {}
            Some(_) => return Err(RemoteError::transport(UNEXPECTED_CHANNEL_REPLY_MESSAGE)),
            None => return Err(RemoteError::transport(CHANNEL_CLOSED_MESSAGE)),
        }
    }
}

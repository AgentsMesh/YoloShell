use super::approved_channel::ApprovalBuffer;
use super::channel_open_custody::SessionChannel;
use super::{RemoteError, ssh_error};
use crate::remote::constants::{
    SFTP_CHANNEL_BRIDGE_BYTES, SFTP_CHANNEL_OPEN_FAILURE_MESSAGE, SFTP_CHANNEL_RECEIVER_EOF_MESSAGE,
};
use russh::ChannelMsg;
use russh::client::Msg;
use tokio::io::{AsyncReadExt, AsyncWriteExt, DuplexStream};
use tokio::sync::oneshot;
use tokio::task::JoinHandle;

pub(super) struct SftpChannelDriver {
    pub(super) close: oneshot::Sender<()>,
    pub(super) task: JoinHandle<Result<(), RemoteError>>,
}

pub(super) fn start(
    channel: SessionChannel,
    approved: ApprovalBuffer,
) -> (DuplexStream, SftpChannelDriver) {
    let (client, bridge) = tokio::io::duplex(SFTP_CHANNEL_BRIDGE_BYTES);
    let (close, requested) = oneshot::channel();
    let task = tokio::spawn(drive(channel, approved, bridge, requested));
    (client, SftpChannelDriver { close, task })
}

enum Finish {
    Local,
    PeerClose,
}

enum Forward {
    Continue,
    StopLocal,
    PeerClose,
    Failed(RemoteError),
}

enum Received {
    Payload(ChannelMsg),
    PeerClose,
    Failed(RemoteError),
}

async fn drive(
    channel: SessionChannel,
    approved: ApprovalBuffer,
    bridge: DuplexStream,
    mut close: oneshot::Receiver<()>,
) -> Result<(), RemoteError> {
    let (mut channel_read, channel_write) = channel.split();
    let (mut client_read, mut client_write) = tokio::io::split(bridge);
    let mut remote_eof = false;
    for message in approved.into_messages() {
        match forward_received(Some(message), &mut client_write, &mut remote_eof).await {
            Forward::Continue => {}
            Forward::StopLocal => {
                return settle(channel_read, channel_write, Finish::Local).await;
            }
            Forward::PeerClose => {
                return settle(channel_read, channel_write, Finish::PeerClose).await;
            }
            Forward::Failed(error) => return Err(error),
        }
    }
    let mut outgoing = vec![u8::default(); SFTP_CHANNEL_BRIDGE_BYTES];
    let finish = loop {
        tokio::select! {
            _ = &mut close => break Finish::Local,
            result = client_read.read(&mut outgoing) => match result {
                Ok(0) | Err(_) => break Finish::Local,
                Ok(count) => {
                    if channel_write
                        .data_bytes(outgoing[..count].to_vec())
                        .await
                        .is_err()
                    {
                        break Finish::Local;
                    }
                }
            },
            message = channel_read.wait() => match forward_received(
                message,
                &mut client_write,
                &mut remote_eof,
            ).await {
                Forward::Continue => {}
                Forward::StopLocal => break Finish::Local,
                Forward::PeerClose => break Finish::PeerClose,
                Forward::Failed(error) => return Err(error),
            },
        }
    };
    drop(client_read);
    drop(client_write);
    settle(channel_read, channel_write, finish).await
}

async fn forward_received<W: tokio::io::AsyncWrite + Unpin>(
    message: Option<ChannelMsg>,
    client: &mut W,
    remote_eof: &mut bool,
) -> Forward {
    match receive(message) {
        Received::Payload(message) => forward_payload(message, client, remote_eof).await,
        Received::PeerClose => Forward::PeerClose,
        Received::Failed(error) => Forward::Failed(error),
    }
}

async fn forward_payload<W: tokio::io::AsyncWrite + Unpin>(
    message: ChannelMsg,
    client: &mut W,
    remote_eof: &mut bool,
) -> Forward {
    match message {
        ChannelMsg::Data { data } if !*remote_eof => {
            if client.write_all(&data).await.is_err() {
                Forward::StopLocal
            } else {
                Forward::Continue
            }
        }
        ChannelMsg::Data { .. } => Forward::StopLocal,
        ChannelMsg::Eof => {
            *remote_eof = true;
            let _ = client.shutdown().await;
            Forward::Continue
        }
        _ => Forward::Continue,
    }
}

fn receive(message: Option<ChannelMsg>) -> Received {
    match message {
        Some(ChannelMsg::Close) => Received::PeerClose,
        Some(ChannelMsg::OpenFailure(_)) => Received::Failed(open_failure()),
        Some(message) => Received::Payload(message),
        None => Received::Failed(receiver_eof()),
    }
}

async fn settle(
    mut read: russh::ChannelReadHalf,
    write: russh::ChannelWriteHalf<Msg>,
    finish: Finish,
) -> Result<(), RemoteError> {
    if matches!(finish, Finish::PeerClose) {
        return Ok(());
    }
    write.close().await.map_err(ssh_error)?;
    loop {
        match receive(read.wait().await) {
            Received::PeerClose => return Ok(()),
            Received::Failed(error) => return Err(error),
            Received::Payload(_) => {}
        }
    }
}

fn open_failure() -> RemoteError {
    RemoteError::transport(SFTP_CHANNEL_OPEN_FAILURE_MESSAGE)
}

fn receiver_eof() -> RemoteError {
    RemoteError::transport(SFTP_CHANNEL_RECEIVER_EOF_MESSAGE)
}

#[cfg(test)]
#[cfg(any(not(test), feature = "yolo-integration-tests"))]
#[path = "../../../integration_tests/russh_backend/sftp_channel_driver_tests.rs"]
mod tests;

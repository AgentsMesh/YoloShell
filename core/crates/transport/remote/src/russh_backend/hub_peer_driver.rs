use super::{approved_channel::ApprovalBuffer, channel_open_custody::SessionChannel, ssh_error};
use crate::remote::RemoteError;
use russh::{ChannelMsg, ChannelReadHalf, ChannelWriteHalf, client::Msg};
use std::collections::VecDeque;
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt, DuplexStream},
    sync::oneshot,
    task::JoinHandle,
};

const BRIDGE_BYTES: usize = 65_536;
const CHANNEL_RECEIVER_CLOSED: &str = "Hub peer channel receiver closed unexpectedly";

pub(super) fn start(
    channel: SessionChannel,
) -> (
    DuplexStream,
    oneshot::Sender<()>,
    JoinHandle<Result<(), RemoteError>>,
) {
    start_buffered(channel, ApprovalBuffer::new())
}

pub(super) fn start_buffered(
    channel: SessionChannel,
    approved: ApprovalBuffer,
) -> (
    DuplexStream,
    oneshot::Sender<()>,
    JoinHandle<Result<(), RemoteError>>,
) {
    let (stream, bridge) = tokio::io::duplex(BRIDGE_BYTES);
    let (close, requested) = oneshot::channel();
    (
        stream,
        close,
        tokio::spawn(drive(channel, bridge, requested, approved.into_messages())),
    )
}

async fn drive(
    channel: SessionChannel,
    bridge: DuplexStream,
    mut close: oneshot::Receiver<()>,
    prefix: VecDeque<ChannelMsg>,
) -> Result<(), RemoteError> {
    let (mut channel_read, channel_write) = channel.split();
    let (mut read, mut write) = tokio::io::split(bridge);
    // Close remains selectable while either direction is blocked by backpressure.
    let peer_closed = {
        let outgoing = async {
            let mut bytes = vec![u8::default(); BRIDGE_BYTES];
            loop {
                let count = match read.read(&mut bytes).await {
                    Ok(count) => count,
                    Err(_) => return Ok::<bool, RemoteError>(false),
                };
                if count == 0 {
                    return Ok(false);
                }
                channel_write
                    .data_bytes(bytes[..count].to_vec())
                    .await
                    .map_err(ssh_error)?;
            }
        };
        let incoming = incoming(&mut channel_read, &mut write, prefix);
        tokio::pin!(outgoing, incoming);
        tokio::select! {
            _ = &mut close => false,
            result = &mut outgoing => result?,
            result = &mut incoming => result?,
        }
    };
    drop((read, write));
    if !peer_closed {
        settle(&mut channel_read, &channel_write).await?;
    }
    Ok(())
}

async fn incoming<W: tokio::io::AsyncWrite + Unpin>(
    read: &mut ChannelReadHalf,
    write: &mut W,
    mut prefix: VecDeque<ChannelMsg>,
) -> Result<bool, RemoteError> {
    let mut eof = false;
    loop {
        let message = match prefix.pop_front() {
            Some(message) => Some(message),
            None => read.wait().await,
        };
        match message {
            Some(ChannelMsg::Data { data }) if !eof => {
                if write.write_all(&data).await.is_err() {
                    return Ok(false);
                }
            }
            Some(ChannelMsg::Data { .. }) => return Ok(false),
            Some(ChannelMsg::Eof) => {
                eof = true;
                let _ = write.shutdown().await;
            }
            Some(ChannelMsg::Close) => return Ok(true),
            Some(ChannelMsg::OpenFailure(_)) => return Err(RemoteError::ChannelRejected),
            Some(_) => {}
            None => return Err(RemoteError::transport(CHANNEL_RECEIVER_CLOSED)),
        }
    }
}

async fn settle(
    read: &mut ChannelReadHalf,
    write: &ChannelWriteHalf<Msg>,
) -> Result<(), RemoteError> {
    write.close().await.map_err(ssh_error)?;
    while let Some(message) = read.wait().await {
        if matches!(message, ChannelMsg::Close) {
            return Ok(());
        }
    }
    Ok(())
}

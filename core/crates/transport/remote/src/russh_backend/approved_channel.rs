use super::super::constants::{MAX_CHANNEL_APPROVAL_BYTES, MAX_CHANNEL_APPROVAL_MESSAGES};
use super::RemoteError;
use russh::ChannelMsg;
use std::collections::VecDeque;
use std::io;
use std::pin::Pin;
use std::task::{Context, Poll};
use tokio::io::{AsyncRead, AsyncWrite, ReadBuf};

pub(super) struct ApprovalBuffer {
    messages: VecDeque<ChannelMsg>,
    bytes: usize,
}

impl ApprovalBuffer {
    pub(super) fn new() -> Self {
        Self {
            messages: VecDeque::new(),
            bytes: 0,
        }
    }

    pub(super) fn push(&mut self, message: ChannelMsg) -> Result<(), RemoteError> {
        let bytes = owned_payload_bytes(&message)?;
        let total = self
            .bytes
            .checked_add(bytes)
            .ok_or(RemoteError::LimitExceeded)?;
        if total > MAX_CHANNEL_APPROVAL_BYTES
            || self.messages.len() >= MAX_CHANNEL_APPROVAL_MESSAGES
        {
            return Err(RemoteError::LimitExceeded);
        }
        self.bytes = total;
        self.messages.push_back(message);
        Ok(())
    }

    pub(super) async fn next(
        &mut self,
        channel: &mut russh::Channel<russh::client::Msg>,
    ) -> Option<ChannelMsg> {
        match self.messages.pop_front() {
            Some(message) => Some(message),
            None => channel.wait().await,
        }
    }

    pub(super) fn into_stream(
        self,
        channel: russh::Channel<russh::client::Msg>,
    ) -> ApprovedChannelStream {
        let (prefix, eof) = replay_messages(self.messages);
        ApprovedChannelStream {
            prefix,
            offset: 0,
            eof,
            inner: channel.into_stream(),
        }
    }

    pub(super) fn into_messages(self) -> VecDeque<ChannelMsg> {
        self.messages
    }
}

fn owned_payload_bytes(message: &ChannelMsg) -> Result<usize, RemoteError> {
    match message {
        ChannelMsg::Data { data } | ChannelMsg::ExtendedData { data, .. } => Ok(data.len()),
        ChannelMsg::ExitSignal {
            signal_name,
            error_message,
            lang_tag,
            ..
        } => checked_payload_sum([
            custom_signal_bytes(signal_name),
            error_message.len(),
            lang_tag.len(),
        ]),
        _ => Ok(0),
    }
}

fn custom_signal_bytes(signal: &russh::Sig) -> usize {
    match signal {
        russh::Sig::Custom(name) => name.len(),
        _ => 0,
    }
}

fn checked_payload_sum(lengths: impl IntoIterator<Item = usize>) -> Result<usize, RemoteError> {
    lengths.into_iter().try_fold(0_usize, |total, length| {
        total.checked_add(length).ok_or(RemoteError::LimitExceeded)
    })
}

pub(super) struct ApprovedChannelStream {
    prefix: VecDeque<Vec<u8>>,
    offset: usize,
    eof: bool,
    inner: russh::ChannelStream<russh::client::Msg>,
}

impl AsyncRead for ApprovedChannelStream {
    fn poll_read(
        mut self: Pin<&mut Self>,
        context: &mut Context<'_>,
        target: &mut ReadBuf<'_>,
    ) -> Poll<io::Result<()>> {
        if target.remaining() == 0 {
            return Poll::Ready(Ok(()));
        }
        let stream = &mut *self;
        if replay_prefix(&mut stream.prefix, &mut stream.offset, target) {
            return Poll::Ready(Ok(()));
        }
        if self.eof {
            return Poll::Ready(Ok(()));
        }
        Pin::new(&mut self.inner).poll_read(context, target)
    }
}

impl AsyncWrite for ApprovedChannelStream {
    fn poll_write(
        mut self: Pin<&mut Self>,
        context: &mut Context<'_>,
        data: &[u8],
    ) -> Poll<io::Result<usize>> {
        Pin::new(&mut self.inner).poll_write(context, data)
    }

    fn poll_flush(mut self: Pin<&mut Self>, context: &mut Context<'_>) -> Poll<io::Result<()>> {
        Pin::new(&mut self.inner).poll_flush(context)
    }

    fn poll_shutdown(mut self: Pin<&mut Self>, context: &mut Context<'_>) -> Poll<io::Result<()>> {
        Pin::new(&mut self.inner).poll_shutdown(context)
    }
}

fn replay_messages(messages: VecDeque<ChannelMsg>) -> (VecDeque<Vec<u8>>, bool) {
    let mut prefix = VecDeque::new();
    let mut eof = false;
    for message in messages {
        match message {
            ChannelMsg::Data { data } => prefix.push_back(data.to_vec()),
            ChannelMsg::Eof | ChannelMsg::Close => {
                eof = true;
                break;
            }
            _ => {}
        }
    }
    (prefix, eof)
}

fn replay_prefix(
    prefix: &mut VecDeque<Vec<u8>>,
    offset: &mut usize,
    target: &mut ReadBuf<'_>,
) -> bool {
    while let Some(front) = prefix.pop_front() {
        let accepted = target.remaining().min(front.len().saturating_sub(*offset));
        target.put_slice(&front[*offset..*offset + accepted]);
        *offset += accepted;
        if *offset != front.len() {
            prefix.push_front(front);
        } else {
            *offset = 0;
        }
        if accepted != 0 {
            return true;
        }
    }
    false
}

#[cfg(test)]
#[cfg(any(not(test), feature = "yolo-integration-tests"))]
#[path = "../../integration_tests/russh_backend/approved_channel_tests.rs"]
mod tests;

use super::channel_transfer_custody::TransferredChannelCustody;
use std::{
    io,
    pin::Pin,
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
    },
    task::{Context, Poll},
};
use tokio::io::{AsyncRead, AsyncWrite, DuplexStream, ReadBuf};

const CLOSED_CHANNEL: &str = "Hub peer SSH channel is closed";

struct Custody {
    close: Mutex<Option<TransferredChannelCustody>>,
    closed: AtomicBool,
}

/// Explicit close authority for only this accepted SSH channel.
#[derive(Clone)]
pub struct HubPeerClose(Arc<Custody>);

impl HubPeerClose {
    pub fn close(&self) {
        if self.0.closed.swap(true, Ordering::AcqRel) {
            return;
        }
        let custody = self
            .0
            .close
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .take();
        drop(custody);
    }
}

pub struct HubPeerStream {
    inner: DuplexStream,
    close: HubPeerClose,
}

impl HubPeerStream {
    pub(super) fn new(inner: DuplexStream, custody: TransferredChannelCustody) -> Self {
        Self {
            inner,
            close: HubPeerClose(Arc::new(Custody {
                close: Mutex::new(Some(custody)),
                closed: AtomicBool::new(false),
            })),
        }
    }
    pub fn close_handle(&self) -> HubPeerClose {
        self.close.clone()
    }
}

impl Drop for HubPeerStream {
    fn drop(&mut self) {
        self.close.close();
    }
}

impl AsyncRead for HubPeerStream {
    fn poll_read(
        mut self: Pin<&mut Self>,
        context: &mut Context<'_>,
        output: &mut ReadBuf<'_>,
    ) -> Poll<io::Result<()>> {
        if self.close.0.closed.load(Ordering::Acquire) {
            return Poll::Ready(Err(closed()));
        }
        Pin::new(&mut self.inner).poll_read(context, output)
    }
}

impl AsyncWrite for HubPeerStream {
    fn poll_write(
        mut self: Pin<&mut Self>,
        context: &mut Context<'_>,
        bytes: &[u8],
    ) -> Poll<io::Result<usize>> {
        if self.close.0.closed.load(Ordering::Acquire) {
            return Poll::Ready(Err(closed()));
        }
        Pin::new(&mut self.inner).poll_write(context, bytes)
    }
    fn poll_flush(mut self: Pin<&mut Self>, context: &mut Context<'_>) -> Poll<io::Result<()>> {
        if self.close.0.closed.load(Ordering::Acquire) {
            return Poll::Ready(Err(closed()));
        }
        Pin::new(&mut self.inner).poll_flush(context)
    }
    fn poll_shutdown(mut self: Pin<&mut Self>, context: &mut Context<'_>) -> Poll<io::Result<()>> {
        if self.close.0.closed.load(Ordering::Acquire) {
            return Poll::Ready(Ok(()));
        }
        Pin::new(&mut self.inner).poll_shutdown(context)
    }
}

fn closed() -> io::Error {
    io::Error::new(io::ErrorKind::NotConnected, CLOSED_CHANNEL)
}

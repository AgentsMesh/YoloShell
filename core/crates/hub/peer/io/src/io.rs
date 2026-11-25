use crate::custody::{CloseState, IoCustody};
use std::{
    io,
    pin::Pin,
    sync::Arc,
    task::{Context, Poll},
};
use tokio::io::{AsyncRead, AsyncWrite, ReadBuf};

const CLOSED_IO: &str = "Hub peer transport was closed";

/// Any byte stream that can carry one peer link.
pub trait Duplex: AsyncRead + AsyncWrite + Unpin + Send {}
impl<T: AsyncRead + AsyncWrite + Unpin + Send> Duplex for T {}

/// One transport and the authority to close it, independent of its concrete I/O.
pub struct PeerIo {
    stream: Box<dyn Duplex>,
    custody: IoCustody,
}

impl PeerIo {
    /// `close_once` must be nonblocking and must close only this transport.
    /// It runs once on explicit close, handshake failure/cancellation, or drop.
    pub fn new(
        stream: impl Duplex + 'static,
        close_once: impl FnOnce() + Send + Sync + 'static,
    ) -> Self {
        let state = CloseState::new(close_once);
        Self {
            stream: Box::new(ControlledIo {
                inner: Box::new(stream),
                state: state.clone(),
            }),
            custody: IoCustody(state),
        }
    }

    pub fn close(&self) {
        self.custody.close();
    }
}

impl AsyncRead for PeerIo {
    fn poll_read(
        mut self: Pin<&mut Self>,
        context: &mut Context<'_>,
        output: &mut ReadBuf<'_>,
    ) -> Poll<io::Result<()>> {
        Pin::new(&mut self.stream).poll_read(context, output)
    }
}

impl AsyncWrite for PeerIo {
    fn poll_write(
        mut self: Pin<&mut Self>,
        context: &mut Context<'_>,
        bytes: &[u8],
    ) -> Poll<io::Result<usize>> {
        Pin::new(&mut self.stream).poll_write(context, bytes)
    }
    fn poll_flush(mut self: Pin<&mut Self>, context: &mut Context<'_>) -> Poll<io::Result<()>> {
        Pin::new(&mut self.stream).poll_flush(context)
    }
    fn poll_shutdown(mut self: Pin<&mut Self>, context: &mut Context<'_>) -> Poll<io::Result<()>> {
        Pin::new(&mut self.stream).poll_shutdown(context)
    }
}

struct ControlledIo {
    inner: Box<dyn Duplex>,
    state: Arc<CloseState>,
}

impl AsyncRead for ControlledIo {
    fn poll_read(
        mut self: Pin<&mut Self>,
        context: &mut Context<'_>,
        output: &mut ReadBuf<'_>,
    ) -> Poll<io::Result<()>> {
        self.state.reading(context);
        if self.state.is_closed() {
            return Poll::Ready(Err(closed()));
        }
        Pin::new(&mut self.inner).poll_read(context, output)
    }
}

impl AsyncWrite for ControlledIo {
    fn poll_write(
        mut self: Pin<&mut Self>,
        context: &mut Context<'_>,
        bytes: &[u8],
    ) -> Poll<io::Result<usize>> {
        self.state.writing(context);
        if self.state.is_closed() {
            return Poll::Ready(Err(closed()));
        }
        Pin::new(&mut self.inner).poll_write(context, bytes)
    }
    fn poll_flush(mut self: Pin<&mut Self>, context: &mut Context<'_>) -> Poll<io::Result<()>> {
        self.state.writing(context);
        if self.state.is_closed() {
            return Poll::Ready(Err(closed()));
        }
        Pin::new(&mut self.inner).poll_flush(context)
    }
    fn poll_shutdown(mut self: Pin<&mut Self>, context: &mut Context<'_>) -> Poll<io::Result<()>> {
        self.state.writing(context);
        if self.state.is_closed() {
            return Poll::Ready(Ok(()));
        }
        Pin::new(&mut self.inner).poll_shutdown(context)
    }
}

fn closed() -> io::Error {
    io::Error::new(io::ErrorKind::NotConnected, CLOSED_IO)
}

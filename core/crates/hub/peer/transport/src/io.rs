use crate::{
    Duplex,
    io_custody::{CloseState, IoCustody},
};
use std::{
    io,
    pin::Pin,
    sync::Arc,
    task::{Context, Poll},
};
use tokio::io::{AsyncRead, AsyncWrite, ReadBuf};

const CLOSED_IO: &str = "Hub peer transport was closed";

/// One transport and the authority to close it, independent of its concrete I/O.
pub struct PeerIo {
    stream: Box<dyn Duplex>,
    custody: IoCustody,
}

impl PeerIo {
    /// Retains the existing duplicate-socket close authority for TCP transports.
    pub fn from_tcp(stream: tokio::net::TcpStream) -> io::Result<Self> {
        crate::socket::SocketOwner::capture(stream)
    }

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

    pub(crate) fn into_parts(self) -> (Box<dyn Duplex>, IoCustody) {
        (self.stream, self.custody)
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

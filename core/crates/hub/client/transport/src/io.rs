use crate::{
    Duplex,
    constants::CLOSED,
    custody::{CloseState, IoCustody},
};
use std::{
    io,
    net::Shutdown,
    pin::Pin,
    sync::Arc,
    task::{Context, Poll},
};
use tokio::{
    io::{AsyncRead, AsyncWrite, ReadBuf},
    net::{TcpStream, UnixStream},
};

/// Owns one transport even while russh retains a task after its handle is dropped.
pub struct ClientIo {
    stream: Box<dyn Duplex>,
    custody: IoCustody,
}

impl ClientIo {
    /// The callback must be nonblocking and close only this underlying transport.
    /// It runs once on close, drop, or handshake failure/cancellation.
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
    pub fn from_tcp(stream: TcpStream) -> io::Result<Self> {
        let original = stream.into_std()?;
        let retained = original.try_clone()?;
        Ok(Self::new(TcpStream::from_std(original)?, move || {
            let _ = retained.shutdown(Shutdown::Both);
        }))
    }
    pub fn from_unix(stream: UnixStream) -> io::Result<Self> {
        let original = stream.into_std()?;
        let retained = original.try_clone()?;
        Ok(Self::new(UnixStream::from_std(original)?, move || {
            let _ = retained.shutdown(Shutdown::Both);
        }))
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
    io::Error::new(io::ErrorKind::NotConnected, CLOSED)
}

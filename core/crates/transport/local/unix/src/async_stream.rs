use super::constants::POISONED_DETAIL;
use std::io;
use std::os::fd::{AsRawFd, RawFd};
use std::path::Path;
use std::pin::Pin;
use std::task::{Context, Poll, ready};
use tokio::io::{AsyncRead, AsyncWrite, ReadBuf};
use tokio::net::UnixStream;

pub struct LocalUnixStream {
    inner: UnixStream,
    poisoned: bool,
}

impl std::fmt::Debug for LocalUnixStream {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct(stringify!(LocalUnixStream))
            .finish_non_exhaustive()
    }
}

impl LocalUnixStream {
    pub async fn connect(path: impl AsRef<Path>) -> io::Result<Self> {
        UnixStream::connect(path).await.map(Self::from)
    }

    pub fn pair() -> io::Result<(Self, Self)> {
        UnixStream::pair().map(|(left, right)| (Self::from(left), Self::from(right)))
    }

    pub fn from_std(stream: std::os::unix::net::UnixStream) -> io::Result<Self> {
        UnixStream::from_std(stream).map(Self::from)
    }

    pub fn peer_cred(&self) -> io::Result<tokio::net::unix::UCred> {
        self.inner.peer_cred()
    }

    pub fn peer_addr(&self) -> io::Result<tokio::net::unix::SocketAddr> {
        self.inner.peer_addr()
    }

    pub async fn readable(&self) -> io::Result<()> {
        self.ensure_usable()?;
        self.inner.readable().await
    }

    pub async fn writable(&self) -> io::Result<()> {
        self.ensure_usable()?;
        self.inner.writable().await
    }

    pub async fn async_io<R>(
        &self,
        interest: tokio::io::Interest,
        operation: impl FnMut() -> io::Result<R>,
    ) -> io::Result<R> {
        self.ensure_usable()?;
        self.inner.async_io(interest, operation).await
    }

    pub fn try_read(&mut self, destination: &mut [u8]) -> io::Result<usize> {
        self.ensure_usable()?;
        if destination.is_empty() {
            return Ok(usize::default());
        }
        #[cfg(unix)]
        let result = self.inner.try_io(tokio::io::Interest::READABLE, || {
            super::ancillary_recv::receive_rejecting(self.inner.as_raw_fd(), destination)
        });
        #[cfg(not(unix))]
        let result = self.inner.try_read(destination);
        result.map_err(|error| self.poison(error))
    }

    pub fn try_read_kernel(&mut self, destination: &mut [u8]) -> io::Result<usize> {
        self.ensure_usable()?;
        if destination.is_empty() {
            return Ok(usize::default());
        }
        #[cfg(unix)]
        let result = super::ancillary_recv::receive_rejecting(self.inner.as_raw_fd(), destination);
        #[cfg(not(unix))]
        let result = self.inner.try_read(destination);
        result.map_err(|error| self.poison(error))
    }

    pub(crate) fn raw_descriptor(&self) -> RawFd {
        self.inner.as_raw_fd()
    }

    fn poison(&mut self, error: io::Error) -> io::Error {
        if super::is_ancillary_failure(&error) {
            self.poisoned = true;
        }
        error
    }

    fn ensure_usable(&self) -> io::Result<()> {
        if self.poisoned {
            Err(io::Error::new(io::ErrorKind::InvalidData, POISONED_DETAIL))
        } else {
            Ok(())
        }
    }
}

impl From<UnixStream> for LocalUnixStream {
    fn from(inner: UnixStream) -> Self {
        Self {
            inner,
            poisoned: false,
        }
    }
}

impl AsRawFd for LocalUnixStream {
    fn as_raw_fd(&self) -> RawFd {
        self.raw_descriptor()
    }
}

impl AsyncRead for LocalUnixStream {
    fn poll_read(
        mut self: Pin<&mut Self>,
        context: &mut Context<'_>,
        buffer: &mut ReadBuf<'_>,
    ) -> Poll<io::Result<()>> {
        if let Err(error) = self.ensure_usable() {
            return Poll::Ready(Err(error));
        }
        if buffer.remaining() == usize::default() {
            return Poll::Ready(Ok(()));
        }
        #[cfg(unix)]
        loop {
            ready!(self.inner.poll_read_ready(context))?;
            let result = {
                let destination = buffer.initialize_unfilled();
                self.inner.try_io(tokio::io::Interest::READABLE, || {
                    super::ancillary_recv::receive_rejecting(self.inner.as_raw_fd(), destination)
                })
            };
            match result {
                Ok(received) => {
                    buffer.advance(received);
                    return Poll::Ready(Ok(()));
                }
                Err(error) if error.kind() == io::ErrorKind::WouldBlock => continue,
                Err(error) => {
                    let error = self.poison(error);
                    return Poll::Ready(Err(error));
                }
            }
        }
        #[cfg(not(unix))]
        Pin::new(&mut self.inner).poll_read(context, buffer)
    }
}

#[path = "async_stream_write.rs"]
mod write;

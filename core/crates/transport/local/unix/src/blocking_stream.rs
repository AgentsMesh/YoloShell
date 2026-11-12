use super::constants::POISONED_DETAIL;
use std::io::{self, Read, Write};
use std::os::fd::{AsRawFd, FromRawFd, RawFd};
use std::path::Path;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

pub struct BlockingLocalUnixStream {
    inner: std::os::unix::net::UnixStream,
    poisoned: Arc<AtomicBool>,
}

impl BlockingLocalUnixStream {
    pub fn connect(path: impl AsRef<Path>) -> io::Result<Self> {
        std::os::unix::net::UnixStream::connect(path).map(Self::from)
    }

    pub fn pair() -> io::Result<(Self, Self)> {
        std::os::unix::net::UnixStream::pair()
            .map(|(left, right)| (Self::from(left), Self::from(right)))
    }

    pub fn try_clone(&self) -> io::Result<Self> {
        Ok(Self {
            inner: self.inner.try_clone()?,
            poisoned: Arc::clone(&self.poisoned),
        })
    }

    pub fn set_nonblocking(&self, value: bool) -> io::Result<()> {
        self.inner.set_nonblocking(value)
    }

    pub fn set_read_timeout(&self, timeout: Option<std::time::Duration>) -> io::Result<()> {
        self.inner.set_read_timeout(timeout)
    }

    pub fn set_write_timeout(&self, timeout: Option<std::time::Duration>) -> io::Result<()> {
        self.inner.set_write_timeout(timeout)
    }

    fn raw_descriptor(&self) -> RawFd {
        self.inner.as_raw_fd()
    }

    fn ensure_usable(&self) -> io::Result<()> {
        if self.poisoned.load(Ordering::Acquire) {
            Err(io::Error::new(io::ErrorKind::InvalidData, POISONED_DETAIL))
        } else {
            Ok(())
        }
    }
}

impl From<std::os::unix::net::UnixStream> for BlockingLocalUnixStream {
    fn from(inner: std::os::unix::net::UnixStream) -> Self {
        Self {
            inner,
            poisoned: Arc::new(AtomicBool::new(false)),
        }
    }
}

impl AsRawFd for BlockingLocalUnixStream {
    fn as_raw_fd(&self) -> RawFd {
        self.raw_descriptor()
    }
}

impl Read for BlockingLocalUnixStream {
    fn read(&mut self, buffer: &mut [u8]) -> io::Result<usize> {
        #[cfg(unix)]
        {
            self.ensure_usable()?;
            if buffer.is_empty() {
                return Ok(usize::default());
            }
            let deadline = self
                .inner
                .read_timeout()?
                .map(|timeout| std::time::Instant::now() + timeout);
            loop {
                match super::ancillary_recv::receive_rejecting(self.raw_descriptor(), buffer) {
                    Ok(received) => return Ok(received),
                    Err(error) if error.kind() == io::ErrorKind::WouldBlock => {
                        super::blocking_wait::readable(&self.inner, deadline)?;
                    }
                    Err(error) => {
                        if super::is_ancillary_failure(&error) {
                            self.poisoned.store(true, Ordering::Release);
                        }
                        return Err(error);
                    }
                }
            }
        }
        #[cfg(not(unix))]
        self.inner.read(buffer)
    }
}

impl Write for BlockingLocalUnixStream {
    fn write(&mut self, buffer: &[u8]) -> io::Result<usize> {
        self.ensure_usable()?;
        self.inner.write(buffer)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.ensure_usable()?;
        self.inner.flush()
    }
}

impl FromRawFd for BlockingLocalUnixStream {
    unsafe fn from_raw_fd(descriptor: RawFd) -> Self {
        Self::from(unsafe { std::os::unix::net::UnixStream::from_raw_fd(descriptor) })
    }
}

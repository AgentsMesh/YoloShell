use std::io;
use std::os::fd::{AsRawFd, FromRawFd, OwnedFd, RawFd};
use std::pin::Pin;
use std::task::{Context, Poll, ready};
use tokio::io::{AsyncRead, ReadBuf, unix::AsyncFd};

const FAILED: libc::c_int = -1;
const MINIMUM_DUPLICATE: libc::c_int = libc::STDERR_FILENO + 1;

struct NonblockingInput {
    descriptor: OwnedFd,
    original_flags: libc::c_int,
}

impl AsRawFd for NonblockingInput {
    fn as_raw_fd(&self) -> RawFd {
        self.descriptor.as_raw_fd()
    }
}

impl Drop for NonblockingInput {
    fn drop(&mut self) {
        let _ = unsafe { libc::fcntl(self.as_raw_fd(), libc::F_SETFL, self.original_flags) };
    }
}

pub(super) struct TerminalInput(AsyncFd<NonblockingInput>);

impl TerminalInput {
    pub(super) fn new(descriptor: RawFd) -> io::Result<Self> {
        let duplicate =
            unsafe { libc::fcntl(descriptor, libc::F_DUPFD_CLOEXEC, MINIMUM_DUPLICATE) };
        if duplicate == FAILED {
            return Err(io::Error::last_os_error());
        }
        // SAFETY: F_DUPFD_CLOEXEC returned a fresh descriptor owned by this instance.
        let descriptor = unsafe { OwnedFd::from_raw_fd(duplicate) };
        let original_flags = unsafe { libc::fcntl(descriptor.as_raw_fd(), libc::F_GETFL) };
        if original_flags == FAILED {
            return Err(io::Error::last_os_error());
        }
        let input = NonblockingInput {
            descriptor,
            original_flags,
        };
        if unsafe {
            libc::fcntl(
                input.as_raw_fd(),
                libc::F_SETFL,
                original_flags | libc::O_NONBLOCK,
            )
        } == FAILED
        {
            return Err(io::Error::last_os_error());
        }
        AsyncFd::new(input).map(Self)
    }
}

impl AsyncRead for TerminalInput {
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buffer: &mut ReadBuf<'_>,
    ) -> Poll<io::Result<()>> {
        if buffer.remaining() == 0 {
            return Poll::Ready(Ok(()));
        }
        loop {
            let mut ready = ready!(self.0.poll_read_ready(cx))?;
            let result = ready.try_io(|inner| {
                let bytes = buffer.initialize_unfilled();
                let count = unsafe {
                    libc::read(inner.as_raw_fd(), bytes.as_mut_ptr().cast(), bytes.len())
                };
                if count < 0 {
                    Err(io::Error::last_os_error())
                } else {
                    Ok(count as usize)
                }
            });
            match result {
                Ok(Ok(count)) => {
                    buffer.advance(count);
                    return Poll::Ready(Ok(()));
                }
                Ok(Err(error)) if error.kind() == io::ErrorKind::Interrupted => continue,
                Ok(Err(error)) => return Poll::Ready(Err(error)),
                Err(_) => continue,
            }
        }
    }
}

#[cfg(all(test, feature = "yolo-integration-tests"))]
#[path = "../integration_tests/terminal_input_tests.rs"]
mod tests;

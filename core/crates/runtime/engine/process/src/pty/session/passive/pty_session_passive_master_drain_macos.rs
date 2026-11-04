use super::MacosPassivePtyMaster;
use anyhow::{Context, Result, ensure};
use std::io;
use std::os::fd::{AsRawFd, OwnedFd};

const DRAIN_BUFFER_BYTES: usize = 4_096;
const DRAIN_READ_PASSES: usize = 16;
const FCNTL_FAILURE: libc::c_int = -1;
const READ_FAILURE: libc::ssize_t = -1;
const STATUS_FLAGS_CONTEXT: &str = "configure passive PTY master status flags";
const DRAIN_CONTEXT: &str = "drain passive PTY master after namespace loss";

#[cfg(any(
    all(test, feature = "yolo-integration-tests"),
    feature = "adversarial-test-hooks"
))]
#[path = "../../../../test_support/namespace/namespace_loss_drain_hook.rs"]
mod test_hook;
#[cfg(any(
    all(test, feature = "yolo-integration-tests"),
    feature = "adversarial-test-hooks"
))]
pub use test_hook::install_pending_once;

pub(super) fn configure_nonblocking(descriptor: &OwnedFd) -> Result<()> {
    let raw = descriptor.as_raw_fd();
    let flags = unsafe { libc::fcntl(raw, libc::F_GETFL) };
    ensure!(flags != FCNTL_FAILURE, STATUS_FLAGS_CONTEXT);
    if flags & libc::O_NONBLOCK == libc::c_int::default() {
        ensure!(
            unsafe { libc::fcntl(raw, libc::F_SETFL, flags | libc::O_NONBLOCK) } != FCNTL_FAILURE,
            STATUS_FLAGS_CONTEXT
        );
    }
    Ok(())
}

impl MacosPassivePtyMaster {
    pub(in crate::process) fn drain_to_kernel_eof(&self) -> Result<bool> {
        #[cfg(any(
            all(test, feature = "yolo-integration-tests"),
            feature = "adversarial-test-hooks"
        ))]
        if test_hook::take_pending() {
            return Ok(false);
        }
        let mut buffer = [u8::default(); DRAIN_BUFFER_BYTES];
        for _pass in 0..DRAIN_READ_PASSES {
            let read = unsafe {
                libc::read(
                    self.descriptor.as_raw_fd(),
                    buffer.as_mut_ptr().cast(),
                    buffer.len(),
                )
            };
            if read == libc::ssize_t::default() {
                return Ok(true);
            }
            if read > libc::ssize_t::default() {
                continue;
            }
            debug_assert_eq!(read, READ_FAILURE);
            let error = io::Error::last_os_error();
            match error.kind() {
                io::ErrorKind::Interrupted => continue,
                io::ErrorKind::WouldBlock => return Ok(false),
                _ if error.raw_os_error() == Some(libc::EIO) => return Ok(true),
                _ => return Err(error).context(DRAIN_CONTEXT),
            }
        }
        Ok(false)
    }
}

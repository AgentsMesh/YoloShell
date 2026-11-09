use crate::{constants::*, owner_wait::wait_for_owner_loss};
use anyhow::Result;
use std::os::fd::BorrowedFd;
use std::os::unix::net::UnixStream;
use std::time::Instant;
use yolo_hub_file_helper_protocol::{HELPER_EXIT_EXPIRED, MAX_HELPER_LIFETIME};

/// The owner-only channel is independent of both framing and blocking file syscalls.
pub fn watch_owner() -> Result<()> {
    let descriptor = unsafe { BorrowedFd::borrow_raw(libc::STDERR_FILENO) }.try_clone_to_owned()?;
    let stream = UnixStream::from(descriptor);
    stream.peer_addr()?;
    let deadline = Instant::now() + MAX_HELPER_LIFETIME;
    std::thread::Builder::new()
        .name(OWNER_WATCH_THREAD.into())
        .spawn(move || {
            wait_for_owner_loss(stream, deadline);
            unsafe { libc::_exit(HELPER_EXIT_EXPIRED) };
        })?;
    Ok(())
}

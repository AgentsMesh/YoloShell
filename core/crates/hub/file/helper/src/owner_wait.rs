use crate::constants::*;
use std::{
    os::{fd::AsRawFd, unix::net::UnixStream},
    time::Instant,
};

/// Waits independently of request framing; EOF, unexpected data and timeout revoke ownership.
pub fn wait_for_owner_loss(stream: UnixStream, deadline: Instant) {
    let mut descriptor = libc::pollfd {
        fd: stream.as_raw_fd(),
        events: OWNER_EVENTS,
        revents: 0,
    };
    loop {
        let remaining = deadline.saturating_duration_since(Instant::now());
        let milliseconds = remaining.as_millis().min(i32::MAX as u128) as i32;
        let ready = unsafe { libc::poll(&mut descriptor, OWNER_DESCRIPTOR_COUNT, milliseconds) };
        if ready < 0 && std::io::Error::last_os_error().kind() == std::io::ErrorKind::Interrupted {
            continue;
        }
        return;
    }
}

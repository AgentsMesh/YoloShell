use crate::constants::REVOKED;
use anyhow::{Result, ensure};
use std::{
    io::{self, Read, Write},
    os::fd::AsRawFd,
    time::Duration,
};
use tokio::time::Instant;
use yolo_local_unix::BlockingLocalUnixStream;

const ONE_DESCRIPTOR: libc::nfds_t = 1;
const ROUND_UP_MILLISECOND: u128 = 1;
const INVALID_DESCRIPTOR: &str = "launch authority descriptor is invalid";

pub(super) fn prepare(stream: &BlockingLocalUnixStream, deadline: Instant) -> Result<()> {
    require_before(deadline)?;
    // fcntl changes this owned descriptor's mode; it does not touch peer socket options.
    stream.set_nonblocking(true)?;
    require_before(deadline)
}

pub(super) fn read_exact(
    stream: &mut BlockingLocalUnixStream,
    mut bytes: &mut [u8],
    deadline: Instant,
) -> Result<()> {
    while !bytes.is_empty() {
        ready(stream, libc::POLLIN, deadline)?;
        match stream.read(bytes) {
            Ok(count) => {
                ensure!(count != usize::default(), REVOKED);
                bytes = &mut bytes[count..];
            }
            Err(error) if retryable(&error) => continue,
            Err(error) => return Err(error.into()),
        }
    }
    // A complete buffered ACK remains valid after its sender closes. No socket calls here.
    require_before(deadline)
}

pub(super) fn write_all(
    stream: &mut BlockingLocalUnixStream,
    mut bytes: &[u8],
    deadline: Instant,
) -> Result<()> {
    while !bytes.is_empty() {
        ready(stream, libc::POLLOUT, deadline)?;
        match stream.write(bytes) {
            Ok(count) => {
                ensure!(count != usize::default(), REVOKED);
                bytes = &bytes[count..];
            }
            Err(error) if retryable(&error) => continue,
            Err(error) => return Err(error.into()),
        }
    }
    require_before(deadline)
}

fn ready(stream: &BlockingLocalUnixStream, events: libc::c_short, deadline: Instant) -> Result<()> {
    loop {
        let remaining = deadline
            .checked_duration_since(Instant::now())
            .filter(|value| !value.is_zero())
            .ok_or_else(|| anyhow::anyhow!(REVOKED))?;
        let floor = remaining.as_millis();
        let milliseconds = floor.saturating_add(
            u128::from(Duration::from_millis(floor as u64) < remaining) * ROUND_UP_MILLISECOND,
        );
        let timeout = milliseconds.min(libc::c_int::MAX as u128) as libc::c_int;
        let mut descriptor = libc::pollfd {
            fd: stream.as_raw_fd(),
            events,
            revents: libc::c_short::default(),
        };
        // The borrowed stream retains the exact fd for the entire wait; no detached raw fd escapes.
        let result = unsafe { libc::poll(&mut descriptor, ONE_DESCRIPTOR, timeout) };
        let error = (result < libc::c_int::default()).then(io::Error::last_os_error);
        require_before(deadline)?;
        if let Some(error) = error {
            if error.kind() == io::ErrorKind::Interrupted {
                continue;
            }
            return Err(error.into());
        }
        if result == libc::c_int::default() {
            continue;
        }
        ensure!(
            descriptor.revents & libc::POLLNVAL == libc::c_short::default(),
            INVALID_DESCRIPTOR
        );
        if descriptor.revents & (events | libc::POLLHUP | libc::POLLERR) != libc::c_short::default()
        {
            return Ok(());
        }
    }
}

fn retryable(error: &io::Error) -> bool {
    matches!(
        error.kind(),
        io::ErrorKind::WouldBlock | io::ErrorKind::Interrupted
    )
}
fn require_before(deadline: Instant) -> Result<()> {
    ensure!(Instant::now() < deadline, REVOKED);
    Ok(())
}

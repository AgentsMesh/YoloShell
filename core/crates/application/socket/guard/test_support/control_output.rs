use super::constants::*;
use anyhow::{Context, Result, ensure};
use std::io::Read;
use std::os::fd::AsRawFd;
use std::process::ChildStdout;
use std::time::{Duration, Instant};

pub(super) struct ControlOutput {
    output: ChildStdout,
    buffered: Vec<u8>,
}

impl ControlOutput {
    pub(super) fn new(output: ChildStdout) -> Self {
        Self {
            output,
            buffered: Vec::new(),
        }
    }

    pub(super) fn read_frame(&mut self, timeout: Duration) -> Result<String> {
        let deadline = Instant::now() + timeout;
        loop {
            if let Some(frame) = take_frame(&mut self.buffered)? {
                return Ok(frame);
            }
            ensure!(
                u64::try_from(self.buffered.len())? < MAXIMUM_CONTROL_FRAME_BYTES,
                WATCHDOG_PROTOCOL_DETAIL
            );
            ensure!(Instant::now() < deadline, CONTROL_TIMEOUT_DETAIL);
            if !wait_readable(self.output.as_raw_fd(), deadline)? {
                anyhow::bail!(CONTROL_TIMEOUT_DETAIL);
            }
            let mut chunk = [u8::default(); CONTROL_READ_BUFFER_BYTES];
            let read = self.output.read(&mut chunk).context(CONTROL_READ_DETAIL)?;
            ensure!(read > usize::default(), CONTROL_EOF_DETAIL);
            self.buffered.extend_from_slice(&chunk[..read]);
        }
    }
}

fn take_frame(buffered: &mut Vec<u8>) -> Result<Option<String>> {
    let Some(end) = buffered
        .iter()
        .position(|byte| *byte == FRAME_TERMINATOR_BYTE)
    else {
        return Ok(None);
    };
    let frame_bytes = u64::try_from(end)?
        .checked_add(CONTROL_FRAME_OVERFLOW_SENTINEL_BYTES)
        .context(WATCHDOG_PROTOCOL_DETAIL)?;
    ensure!(
        frame_bytes <= MAXIMUM_CONTROL_FRAME_BYTES,
        WATCHDOG_PROTOCOL_DETAIL
    );
    let frame = buffered.drain(..=end).collect::<Vec<_>>();
    String::from_utf8(frame)
        .context(WATCHDOG_PROTOCOL_DETAIL)
        .map(Some)
}

fn wait_readable(descriptor: libc::c_int, deadline: Instant) -> Result<bool> {
    loop {
        let Some(remaining) = deadline.checked_duration_since(Instant::now()) else {
            return Ok(false);
        };
        let milliseconds = remaining
            .as_millis()
            .max(MINIMUM_POLL_MILLISECONDS)
            .min(i32::MAX as u128);
        let mut poll = libc::pollfd {
            fd: descriptor,
            events: libc::POLLIN | libc::POLLHUP,
            revents: libc::c_short::default(),
        };
        let result = unsafe {
            libc::poll(
                &mut poll,
                POLL_DESCRIPTOR_COUNT,
                libc::c_int::try_from(milliseconds)?,
            )
        };
        if result == POLL_READY_COUNT {
            return Ok(true);
        }
        if result == libc::c_int::default() {
            return Ok(false);
        }
        let error = std::io::Error::last_os_error();
        if result == INVALID_SYSTEM_RESULT && error.kind() == std::io::ErrorKind::Interrupted {
            continue;
        }
        return Err(error).context(CONTROL_READ_DETAIL);
    }
}

#[cfg(test)]
#[cfg(any(not(test), feature = "yolo-unit-tests"))] #[path = "../unit_tests/control_output_tests.rs"]
mod tests;

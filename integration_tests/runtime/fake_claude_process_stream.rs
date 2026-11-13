use crate::claude_wire::ClaudeHookFrame;
use std::io::{self, BufRead, BufReader};
use std::os::fd::AsRawFd;
use std::os::unix::net::UnixStream;
use std::time::{Duration, Instant};

const READ_POLL_INTERVAL: Duration = Duration::from_millis(5);
const POLL_DESCRIPTOR_COUNT: libc::nfds_t = 1;
const POLL_READY: libc::c_int = 1;
const POLL_TIMEOUT: libc::c_int = 0;
const READ_TIMEOUT_ERROR: &str = "fake Claude did not finish hook delivery";

pub(super) fn read_frames(
    stream: UnixStream,
    deadline: Instant,
) -> io::Result<Vec<ClaudeHookFrame>> {
    let mut reader = BufReader::new(stream);
    let mut frames = Vec::new();
    loop {
        wait_until_readable(reader.get_ref(), deadline)?;
        let mut line = String::new();
        match reader.read_line(&mut line)? {
            0 => return Ok(frames),
            _ => frames.push(
                serde_json::from_str(&line)
                    .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))?,
            ),
        }
    }
}

fn wait_until_readable(stream: &UnixStream, deadline: Instant) -> io::Result<()> {
    let milliseconds =
        libc::c_int::try_from(READ_POLL_INTERVAL.as_millis()).map_err(io::Error::other)?;
    loop {
        if Instant::now() >= deadline {
            return Err(io::Error::new(io::ErrorKind::TimedOut, READ_TIMEOUT_ERROR));
        }
        let mut descriptor = libc::pollfd {
            fd: stream.as_raw_fd(),
            events: libc::POLLIN | libc::POLLHUP,
            revents: libc::c_short::default(),
        };
        let result = unsafe { libc::poll(&mut descriptor, POLL_DESCRIPTOR_COUNT, milliseconds) };
        match result {
            POLL_READY => return Ok(()),
            POLL_TIMEOUT => {}
            _ => {
                let error = io::Error::last_os_error();
                if error.kind() != io::ErrorKind::Interrupted {
                    return Err(error);
                }
            }
        }
    }
}

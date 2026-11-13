use anyhow::{Context, Result, ensure};
use std::io::{self, Write};
use std::os::fd::AsRawFd;
use std::process::ChildStdin;
use std::time::Instant;

const PRIVATE_INPUT_FLAGS_CONTEXT: &str = "configure detached process private input";
pub(super) use yolo_process_spawn::PRIVATE_INPUT_DEADLINE_CONTEXT;
const POLL_DESCRIPTOR_COUNT: libc::nfds_t = 1;
const NO_POLL_BUDGET_MILLISECONDS: u128 = 0;

pub(super) fn write_private_input_until(
    mut input: ChildStdin,
    private_input: &[u8],
    deadline: Instant,
) -> Result<()> {
    configure_nonblocking(&input).context(PRIVATE_INPUT_FLAGS_CONTEXT)?;
    let mut written = usize::default();
    while written < private_input.len() {
        require_budget(deadline)?;
        match input.write(&private_input[written..]) {
            Ok(0) => return Err(io::Error::from(io::ErrorKind::WriteZero).into()),
            Ok(count) => written += count,
            Err(error) if error.kind() == io::ErrorKind::Interrupted => {}
            Err(error) if error.kind() == io::ErrorKind::WouldBlock => {
                wait_writable(&input, deadline)?;
            }
            Err(error) => return Err(error.into()),
        }
    }
    require_budget(deadline)
}

fn configure_nonblocking(input: &ChildStdin) -> io::Result<()> {
    let descriptor = input.as_raw_fd();
    let flags = unsafe { libc::fcntl(descriptor, libc::F_GETFL) };
    if flags == -1 {
        return Err(io::Error::last_os_error());
    }
    if unsafe { libc::fcntl(descriptor, libc::F_SETFL, flags | libc::O_NONBLOCK) } == -1 {
        return Err(io::Error::last_os_error());
    }
    Ok(())
}

fn wait_writable(input: &ChildStdin, deadline: Instant) -> Result<()> {
    loop {
        let remaining = deadline
            .checked_duration_since(Instant::now())
            .context(PRIVATE_INPUT_DEADLINE_CONTEXT)?;
        ensure!(!remaining.is_zero(), PRIVATE_INPUT_DEADLINE_CONTEXT);
        let milliseconds = poll_milliseconds(remaining)?;
        let mut descriptor = libc::pollfd {
            fd: input.as_raw_fd(),
            events: libc::POLLOUT,
            revents: libc::c_short::default(),
        };
        let status = unsafe { libc::poll(&mut descriptor, POLL_DESCRIPTOR_COUNT, milliseconds) };
        if status > libc::c_int::default() {
            return Ok(());
        }
        if status == libc::c_int::default() {
            return require_budget(deadline);
        }
        let error = io::Error::last_os_error();
        if error.kind() != io::ErrorKind::Interrupted {
            return Err(error.into());
        }
    }
}

fn poll_milliseconds(remaining: std::time::Duration) -> Result<libc::c_int> {
    let milliseconds = remaining.as_millis().min(libc::c_int::MAX as u128);
    ensure!(
        milliseconds > NO_POLL_BUDGET_MILLISECONDS,
        PRIVATE_INPUT_DEADLINE_CONTEXT
    );
    Ok(milliseconds as libc::c_int)
}

#[cfg(test)]
pub(super) fn poll_milliseconds_for_test(remaining: std::time::Duration) -> Result<libc::c_int> {
    poll_milliseconds(remaining)
}

pub fn require_budget(deadline: Instant) -> Result<()> {
    ensure!(Instant::now() < deadline, PRIVATE_INPUT_DEADLINE_CONTEXT);
    Ok(())
}

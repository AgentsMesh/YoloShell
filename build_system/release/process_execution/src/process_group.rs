use anyhow::{Context, Result};

const SIGNAL_ERROR: libc::c_int = -1;

pub(crate) fn signal(group: libc::pid_t, signal: libc::c_int) -> Result<()> {
    if unsafe { libc::kill(-group, signal) } != SIGNAL_ERROR {
        return Ok(());
    }
    let error = std::io::Error::last_os_error();
    if error.raw_os_error() == Some(libc::ESRCH) {
        return Ok(());
    }
    Err(error).context("terminate child process group")
}

pub(crate) fn signal_process(process: libc::pid_t, signal: libc::c_int) -> Result<()> {
    if unsafe { libc::kill(process, signal) } != SIGNAL_ERROR {
        return Ok(());
    }
    let error = std::io::Error::last_os_error();
    if error.raw_os_error() == Some(libc::ESRCH) {
        return Ok(());
    }
    Err(error).context("terminate direct child process")
}

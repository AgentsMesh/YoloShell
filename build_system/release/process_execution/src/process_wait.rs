use crate::constants::WAIT_STATUS_ERROR;
use anyhow::{Context, Result};

pub(crate) fn exited(process: libc::pid_t) -> Result<bool> {
    let mut information = unsafe { std::mem::zeroed::<libc::siginfo_t>() };
    loop {
        let result = unsafe {
            libc::waitid(
                libc::P_PID,
                process as libc::id_t,
                &mut information,
                libc::WEXITED | libc::WNOHANG | libc::WNOWAIT,
            )
        };
        if result != WAIT_STATUS_ERROR {
            return Ok(information.si_pid == process);
        }
        let error = std::io::Error::last_os_error();
        if error.kind() != std::io::ErrorKind::Interrupted {
            return Err(error).context("observe retained child exit");
        }
    }
}

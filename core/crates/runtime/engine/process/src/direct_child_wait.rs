use anyhow::{Context, Result, anyhow};
use std::io;
use std::os::unix::process::ExitStatusExt;

const WAIT_FOR_EXIT_OPTIONS: libc::c_int = 0;
const WAITPID_FAILURE: libc::pid_t = -1;
const DIRECT_CHILD_PID_CONTEXT: &str = "direct PTY child PID is outside waitpid range";
const DIRECT_CHILD_WAIT_CONTEXT: &str = "wait for direct PTY child";
const UNEXPECTED_WAIT_TARGET_DETAIL: &str = "waitpid returned an unexpected direct child PID";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DirectChildExitStatus {
    code: Option<i32>,
    signal: Option<i32>,
}

impl DirectChildExitStatus {
    pub fn code(self) -> Option<i32> {
        self.code
    }

    pub fn signal(self) -> Option<i32> {
        self.signal
    }

    fn from_wait_status(raw_status: libc::c_int) -> Self {
        let status = std::process::ExitStatus::from_raw(raw_status);
        Self {
            code: status.code(),
            signal: status.signal(),
        }
    }
}

pub fn wait(process_id: u32) -> Result<DirectChildExitStatus> {
    let expected = checked_pid(process_id)?;
    loop {
        let mut raw_status = 0;
        let waited = unsafe { libc::waitpid(expected, &mut raw_status, WAIT_FOR_EXIT_OPTIONS) };
        if waited == expected {
            return Ok(DirectChildExitStatus::from_wait_status(raw_status));
        }
        if waited != WAITPID_FAILURE {
            return Err(anyhow!(UNEXPECTED_WAIT_TARGET_DETAIL));
        }
        let error = io::Error::last_os_error();
        if error.kind() != io::ErrorKind::Interrupted {
            return Err(error).context(DIRECT_CHILD_WAIT_CONTEXT);
        }
    }
}

#[cfg(test)]
#[cfg(any(not(test), feature = "yolo-integration-tests"))]
#[path = "../integration_tests/direct_child_wait_tests.rs"]
mod tests;

fn checked_pid(process_id: u32) -> Result<libc::pid_t> {
    let expected = libc::pid_t::try_from(process_id).context(DIRECT_CHILD_PID_CONTEXT)?;
    anyhow::ensure!(expected > 0, DIRECT_CHILD_PID_CONTEXT);
    Ok(expected)
}

use super::{RootWaitAuthority, UnreapedRootExit};
use anyhow::{Context, Result, bail, ensure};
use std::mem::MaybeUninit;

const UNKNOWN_CHILD_EVENT_DETAIL: &str = "waitid observed an unknown PTY child event code";

impl RootWaitAuthority {
    pub fn poll_exited_unreaped(&self) -> Result<Option<UnreapedRootExit>> {
        if self.is_waited() {
            return Ok(None);
        }
        let pid = libc::id_t::try_from(self.root.pid).context(super::ROOT_PID_RANGE_CONTEXT)?;
        let mut information = MaybeUninit::<libc::siginfo_t>::zeroed();
        let result = unsafe {
            libc::waitid(
                libc::P_PID,
                pid,
                information.as_mut_ptr(),
                super::WAITID_OPTIONS,
            )
        };
        if result == super::WAITID_FAILURE_RESULT {
            let error = std::io::Error::last_os_error();
            if error.kind() == std::io::ErrorKind::Interrupted {
                return Ok(None);
            }
            return Err(error).context(super::WAITID_CONTEXT);
        }
        let information = unsafe { information.assume_init() };
        let observed = unsafe { information.si_pid() };
        if observed == super::NO_EXIT_PID {
            return Ok(None);
        }
        ensure!(
            observed == self.root.pid as libc::pid_t,
            "{} {observed}",
            super::UNEXPECTED_CHILD_DETAIL
        );
        if !event_is_exit(information.si_code)? {
            return Ok(None);
        }
        // WNOWAIT keeps this direct-child PID unreusable until the owner consumes the proof.
        Ok(Some(UnreapedRootExit {
            root: self.root,
            live_root: self.live_root.key(),
        }))
    }
}

fn event_is_exit(code: libc::c_int) -> Result<bool> {
    match code {
        libc::CLD_EXITED | libc::CLD_KILLED | libc::CLD_DUMPED => Ok(true),
        libc::CLD_STOPPED | libc::CLD_TRAPPED | libc::CLD_CONTINUED => Ok(false),
        _ => bail!("{UNKNOWN_CHILD_EVENT_DETAIL}: {code}"),
    }
}

#[cfg(test)]
#[cfg(any(not(test), feature = "yolo-integration-tests"))]
#[path = "../../integration_tests/unreaped/unreaped_root_exit_observe_tests.rs"]
mod tests;

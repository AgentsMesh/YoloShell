use super::*;

pub(crate) const EXEC_ORIGIN_CONTEXT: &str =
    "bootstrap exec event does not match the original thread";
pub(crate) const EXEC_EVENT_MESSAGE_CONTEXT: &str = "read bootstrap exec event thread identity";

pub(crate) struct TraceWait {
    pub(crate) target: BootstrapExecTarget,
    pub(crate) thread: libc::pid_t,
    pub(crate) leader: libc::pid_t,
}

impl TraceWait {
    pub(crate) fn new(target: BootstrapExecTarget) -> Result<Self> {
        Ok(Self {
            target,
            thread: platform_pid(target.thread_id)?,
            leader: platform_pid(target.process.pid)?,
        })
    }

    pub(crate) fn thread_id(&self) -> libc::pid_t {
        self.thread
    }

    pub(crate) fn poll(&mut self) -> std::io::Result<(libc::pid_t, libc::c_int)> {
        match wait_thread(self.thread) {
            Err(error)
                if error.raw_os_error() == Some(libc::ECHILD) && self.thread != self.leader =>
            {
                // Non-leader exec adopts the TGID. __WNOTHREAD excludes ordinary children
                // owned by other threads; this observer thread only owns its seized tracee.
                self.thread = self.leader;
                wait_thread(self.thread)
            }
            result => result,
        }
    }

    pub(crate) fn confirm_exec(&self, waited: libc::pid_t) -> Result<()> {
        ensure!(waited == self.leader, EXEC_ORIGIN_CONTEXT);
        let former_thread = ptrace_control(PtraceOperation::ExecThread, waited)
            .context(EXEC_EVENT_MESSAGE_CONTEXT)?;
        ensure!(
            former_thread == libc::c_ulong::from(self.target.thread_id),
            EXEC_ORIGIN_CONTEXT
        );
        ensure!(
            self.target.process.matches(),
            BOOTSTRAP_EXEC_OBSERVER_IDENTITY_CONTEXT
        );
        Ok(())
    }
}

pub(crate) fn wait_thread(thread: libc::pid_t) -> std::io::Result<(libc::pid_t, libc::c_int)> {
    let mut status = 0;
    // SAFETY: status is writable; the pid is exact and options exclude other owners.
    let waited = unsafe { libc::waitpid(thread, &mut status, WAIT_OPTIONS) };
    if waited == INVALID_SYSTEM_RESULT as libc::pid_t {
        Err(std::io::Error::last_os_error())
    } else {
        Ok((waited, status))
    }
}

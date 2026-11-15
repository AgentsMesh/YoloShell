use super::*;
use anyhow::{Context, ensure};
use std::os::fd::{AsRawFd, OwnedFd};
use std::time::Instant;

pub(crate) const INVALID_SYSTEM_RESULT: libc::c_int = -1;
pub(crate) const NO_EVENTS: libc::c_int = 0;
pub(crate) const NO_EVENT_DATA: libc::intptr_t = 0;
pub(crate) const SINGLE_EVENT: libc::c_int = 1;

pub(crate) struct ExecObserver {
    pub(crate) queue: OwnedFd,
    pub(crate) pid: u32,
}

impl ExecObserver {
    pub(crate) fn register(target: BootstrapExecTarget) -> Result<Self> {
        ensure!(
            target.process.pid == target.thread_id,
            BOOTSTRAP_EXEC_THREAD_CONTEXT
        );
        let queue = crate::process::macos_private_kqueue(BOOTSTRAP_EXEC_OBSERVER_REGISTER_CONTEXT)?;
        let change = process_event(
            target.process.pid,
            libc::EV_ADD | libc::EV_ONESHOT,
            observed_notes(),
        );
        // SAFETY: change points to one initialized event; no output is requested.
        let result = unsafe {
            libc::kevent(
                queue.as_raw_fd(),
                &change,
                SINGLE_EVENT,
                std::ptr::null_mut(),
                NO_EVENTS,
                std::ptr::null(),
            )
        };
        if result == INVALID_SYSTEM_RESULT {
            return Err(std::io::Error::last_os_error())
                .context(BOOTSTRAP_EXEC_OBSERVER_REGISTER_CONTEXT);
        }
        ensure!(
            result == NO_EVENTS,
            BOOTSTRAP_EXEC_OBSERVER_REGISTER_CONTEXT
        );
        Ok(Self {
            queue,
            pid: target.process.pid,
        })
    }

    pub(crate) fn wait_until(self, deadline: Instant) -> Result<()> {
        loop {
            let timeout = deadline.saturating_duration_since(Instant::now());
            let timeout = libc::timespec {
                tv_sec: timeout.as_secs().try_into()?,
                tv_nsec: timeout.subsec_nanos().into(),
            };
            let mut event = process_event(self.pid, NO_EVENTS as u16, NO_EVENTS as u32);
            // SAFETY: event and timeout are initialized and remain live for the call.
            let result = unsafe {
                libc::kevent(
                    self.queue.as_raw_fd(),
                    std::ptr::null(),
                    NO_EVENTS,
                    &mut event,
                    SINGLE_EVENT,
                    &timeout,
                )
            };
            if result == INVALID_SYSTEM_RESULT {
                let error = std::io::Error::last_os_error();
                if error.kind() == std::io::ErrorKind::Interrupted {
                    continue;
                }
                return Err(error).context(BOOTSTRAP_EXEC_OBSERVER_WAIT_CONTEXT);
            }
            ensure!(
                result == SINGLE_EVENT,
                BOOTSTRAP_EXEC_OBSERVER_TIMEOUT_CONTEXT
            );
            ensure!(
                event.ident == self.pid as usize,
                BOOTSTRAP_EXEC_OBSERVER_EVENT_CONTEXT
            );
            ensure!(
                event.filter == libc::EVFILT_PROC,
                BOOTSTRAP_EXEC_OBSERVER_EVENT_CONTEXT
            );
            ensure!(
                event.flags & libc::EV_ERROR == u16::default(),
                BOOTSTRAP_EXEC_OBSERVER_EVENT_CONTEXT
            );
            ensure!(
                event.fflags & libc::NOTE_EXEC != u32::default(),
                BOOTSTRAP_EXEC_NOT_OBSERVED_CONTEXT
            );
            return Ok(());
        }
    }
}

pub(crate) fn process_event(pid: u32, flags: u16, fflags: u32) -> libc::kevent {
    libc::kevent {
        ident: pid as usize,
        filter: libc::EVFILT_PROC,
        flags,
        fflags,
        data: NO_EVENT_DATA,
        udata: std::ptr::null_mut(),
    }
}

pub(crate) fn observed_notes() -> u32 {
    libc::NOTE_EXEC | libc::NOTE_EXIT
}

#[cfg(test)]
#[cfg(any(not(test), feature = "yolo-unit-tests"))]
#[cfg(any(feature = "yolo-integration-tests", feature = "yolo-worker-unit-bootstrap"))]
#[path = "../../unit_tests/bootstrap/bootstrap_exec_observer_tests.rs"]
mod tests;

use super::{LiveProcessIdentity, macos_private_kqueue};
use anyhow::{Context, Result, ensure};
use std::os::fd::{AsRawFd, OwnedFd};
use std::time::Instant;

const INVALID_SYSTEM_RESULT: libc::c_int = -1;
const NO_EVENTS: libc::c_int = 0;
const NO_EVENT_DATA: libc::intptr_t = 0;
const SINGLE_EVENT: libc::c_int = 1;
const REGISTER_CONTEXT: &str = "register exact macOS exec observer";
const REGISTER_IDENTITY_DETAIL: &str = "exec target changed while registering observer";
const WAIT_CONTEXT: &str = "wait for exact macOS exec event";
const WAIT_TIMEOUT_DETAIL: &str = "exact macOS exec observation timed out";
const EVENT_DETAIL: &str = "invalid exact macOS exec event";
const EXIT_DETAIL: &str = "exec target exited before executable attestation";
const IDENTITY_DETAIL: &str = "exec target changed before executable attestation";

pub struct MacosExecObserver {
    queue: OwnedFd,
    target: LiveProcessIdentity,
}

impl MacosExecObserver {
    pub(crate) fn register(target: LiveProcessIdentity) -> Result<Self> {
        ensure!(target.try_matches()?, REGISTER_IDENTITY_DETAIL);
        let queue = macos_private_kqueue(REGISTER_CONTEXT)?;
        let change = process_event(
            target.pid(),
            libc::EV_ADD | libc::EV_ONESHOT,
            observed_notes(),
        );
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
            return Err(std::io::Error::last_os_error()).context(REGISTER_CONTEXT);
        }
        ensure!(result == NO_EVENTS, REGISTER_CONTEXT);
        ensure!(target.try_matches()?, REGISTER_IDENTITY_DETAIL);
        Ok(Self { queue, target })
    }

    pub fn wait_until(self, deadline: Instant) -> Result<LiveProcessIdentity> {
        loop {
            ensure_deadline_open(deadline)?;
            let timeout = deadline.saturating_duration_since(Instant::now());
            let timeout = libc::timespec {
                tv_sec: timeout.as_secs().try_into()?,
                tv_nsec: timeout.subsec_nanos().into(),
            };
            let mut event = process_event(self.target.pid(), u16::default(), u32::default());
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
                return Err(error).context(WAIT_CONTEXT);
            }
            ensure!(result == SINGLE_EVENT, WAIT_TIMEOUT_DETAIL);
            run_after_event_hook();
            ensure_deadline_open(deadline)?;
            validate_event(&event, &self.target)?;
            return Ok(self.target);
        }
    }

    #[cfg(any(test, feature = "adversarial-test-hooks"))]
    #[doc(hidden)]
    pub fn install_after_event_hook(
        hook: impl FnOnce() + 'static,
    ) -> test_support::AfterEventHookGuard {
        test_support::install_after_event_hook(hook)
    }
}

fn ensure_deadline_open(deadline: Instant) -> Result<()> {
    ensure!(Instant::now() < deadline, WAIT_TIMEOUT_DETAIL);
    Ok(())
}

#[cfg(any(test, feature = "adversarial-test-hooks"))]
fn run_after_event_hook() {
    test_support::run_after_event_hook();
}

#[cfg(not(any(test, feature = "adversarial-test-hooks")))]
fn run_after_event_hook() {}

fn validate_event(event: &libc::kevent, target: &LiveProcessIdentity) -> Result<()> {
    ensure!(event.ident == target.pid() as usize, EVENT_DETAIL);
    ensure!(event.filter == libc::EVFILT_PROC, EVENT_DETAIL);
    ensure!(event.flags & libc::EV_ERROR == u16::default(), EVENT_DETAIL);
    ensure!(
        event.fflags & libc::NOTE_EXIT == u32::default(),
        EXIT_DETAIL
    );
    ensure!(
        event.fflags & libc::NOTE_EXEC != u32::default(),
        EVENT_DETAIL
    );
    ensure!(target.try_matches()?, IDENTITY_DETAIL);
    Ok(())
}

fn process_event(pid: u32, flags: u16, fflags: u32) -> libc::kevent {
    libc::kevent {
        ident: pid as usize,
        filter: libc::EVFILT_PROC,
        flags,
        fflags,
        data: NO_EVENT_DATA,
        udata: std::ptr::null_mut(),
    }
}

fn observed_notes() -> u32 {
    libc::NOTE_EXEC | libc::NOTE_EXIT
}

#[cfg(any(
    all(test, feature = "yolo-test-support"),
    all(feature = "adversarial-test-hooks", not(test))
))]
#[path = "../../test_support/macos/macos_exec_observer.rs"]
mod test_support;
#[cfg(test)]
#[cfg(any(not(test), feature = "yolo-integration-tests"))]
#[path = "../../integration_tests/macos/macos_exec_observer_tests.rs"]
mod tests;

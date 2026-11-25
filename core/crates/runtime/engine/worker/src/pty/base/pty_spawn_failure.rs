use super::process;
use std::time::Instant;

pub(crate) const CUSTODY_FAIL_STOP_DIAGNOSTIC: &[u8] =
    b"Yolo worker lost unidentified startup child custody; terminating for recovery.\n";
pub(crate) const TERMINATION_ERROR_DETAIL: &str =
    "fresh PTY child termination failed after identity capture failure";
pub(crate) const TERMINATION_PANIC_DETAIL: &str =
    "fresh PTY child termination panicked after identity capture failure";
pub(crate) const POLL_ERROR_DETAIL: &str =
    "fresh PTY child poll failed after identity capture failure";
pub(crate) const POLL_PANIC_DETAIL: &str =
    "fresh PTY child poll panicked after identity capture failure";

pub(crate) fn settle(
    mut child: process::UnidentifiedPtyChildCustody<'_, '_>,
    identity_error: anyhow::Error,
) -> anyhow::Error {
    settle_child(&mut child);
    identity_error
}

pub(crate) fn settle_child(child: &mut process::UnidentifiedPtyChildCustody<'_, '_>) {
    let deadline = Instant::now() + super::constants::STARTUP_CHILD_EXIT_TIMEOUT;
    let mut termination_requested = false;
    loop {
        if !termination_requested {
            termination_requested = request_termination(child);
        }
        if child_settled(child) {
            return;
        }
        if Instant::now() >= deadline {
            fail_stop();
        }
        std::thread::sleep(super::constants::STARTUP_CHILD_EXIT_POLL_INTERVAL);
    }
}

pub(crate) fn fail_stop() -> ! {
    unsafe {
        libc::write(
            libc::STDERR_FILENO,
            CUSTODY_FAIL_STOP_DIAGNOSTIC.as_ptr().cast(),
            CUSTODY_FAIL_STOP_DIAGNOSTIC.len(),
        );
        libc::_exit(libc::EXIT_FAILURE);
    }
}

pub(crate) fn request_termination(
    child: &mut process::UnidentifiedPtyChildCustody<'_, '_>,
) -> bool {
    match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        child.request_direct_termination()
    })) {
        Ok(Ok(())) => true,
        Ok(Err(error)) => {
            super::terminal_reporting::warn_error(error.into(), TERMINATION_ERROR_DETAIL);
            false
        }
        Err(payload) => {
            let detail = super::supervision::panic_detail(payload);
            super::terminal_reporting::warn_detail(&detail, TERMINATION_PANIC_DETAIL);
            false
        }
    }
}

pub(crate) fn child_settled(child: &mut process::UnidentifiedPtyChildCustody<'_, '_>) -> bool {
    match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| child.try_wait())) {
        Ok(Ok(Some(_))) => true,
        Ok(Ok(None)) => false,
        Ok(Err(error)) if process::is_child_already_reaped(&error) => true,
        Ok(Err(error)) => {
            super::terminal_reporting::warn_error(error.into(), POLL_ERROR_DETAIL);
            false
        }
        Err(payload) => {
            let detail = super::supervision::panic_detail(payload);
            super::terminal_reporting::warn_detail(&detail, POLL_PANIC_DETAIL);
            false
        }
    }
}

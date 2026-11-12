use super::super::supervision;
use super::reservation::{SharedReservation, lock_state};
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::sync::Arc;

pub(crate) const CLEANUP_PANIC_DETAIL: &str =
    "startup cleanup task panicked; retrying retained custody";
#[cfg(test)]
pub(crate) const INJECTED_REAPER_PANIC: &str = "injected startup cleanup reaper panic";

pub(crate) fn run_reserved(shared: Arc<SharedReservation>) {
    #[cfg(test)]
    let startup_panicked = catch_unwind(AssertUnwindSafe(|| {
        assert!(!shared.panic_on_start, "{INJECTED_REAPER_PANIC}");
    }))
    .err()
    .is_some_and(|payload| {
        let _ = supervision::panic_detail(payload);
        true
    });
    if let Some(mut cleanup) = wait_for_handoff(&shared) {
        run_cleanup(cleanup.as_mut());
    }
    #[cfg(test)]
    if let Some(thread_exit) = &shared.thread_exit {
        thread_exit.store(true, std::sync::atomic::Ordering::Release);
    }
    #[cfg(test)]
    assert!(!startup_panicked, "{INJECTED_REAPER_PANIC}");
}

pub(crate) fn run_cleanup(cleanup: &mut dyn FnMut()) {
    loop {
        match catch_unwind(AssertUnwindSafe(&mut *cleanup)) {
            Ok(()) => return,
            Err(payload) => {
                let detail = supervision::panic_detail(payload);
                super::super::terminal_reporting::warn_detail(&detail, CLEANUP_PANIC_DETAIL);
                std::thread::sleep(super::super::constants::STARTUP_CHILD_EXIT_POLL_INTERVAL);
            }
        }
    }
}

pub(crate) fn wait_for_handoff(shared: &SharedReservation) -> Option<Box<dyn FnMut() + Send>> {
    let mut state = lock_state(shared);
    while !state.closed {
        state = shared
            .ready
            .wait(state)
            .unwrap_or_else(std::sync::PoisonError::into_inner);
    }
    state.cleanup.take()
}

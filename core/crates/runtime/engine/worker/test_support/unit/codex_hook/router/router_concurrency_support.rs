use super::support::SINGLE_ENTRY_COUNT;
use std::sync::{Condvar, Mutex};
use std::time::Duration;

const CONCURRENCY_TIMEOUT_SECONDS: u64 = 5;
pub(super) const CONCURRENCY_TIMEOUT: Duration = Duration::from_secs(CONCURRENCY_TIMEOUT_SECONDS);

#[derive(Default)]
pub(super) struct BuilderPause {
    state: Mutex<BuilderPauseState>,
    changed: Condvar,
}

#[derive(Default)]
struct BuilderPauseState {
    entered: usize,
    released: bool,
}

impl BuilderPause {
    pub(super) fn block(&self) {
        let mut state = self.state.lock().unwrap();
        state.entered = state.entered.checked_add(SINGLE_ENTRY_COUNT).unwrap();
        self.changed.notify_all();
        while !state.released {
            state = self.changed.wait(state).unwrap();
        }
    }

    pub(super) fn wait_until_entered(&self) {
        assert!(self.wait_for_entries(SINGLE_ENTRY_COUNT));
    }

    pub(super) fn wait_for_entries(&self, expected: usize) -> bool {
        let state = self.state.lock().unwrap();
        let (state, result) = self
            .changed
            .wait_timeout_while(state, CONCURRENCY_TIMEOUT, |state| state.entered < expected)
            .unwrap();
        !result.timed_out() || state.entered >= expected
    }

    pub(super) fn release(&self) {
        let mut state = self.state.lock().unwrap();
        state.released = true;
        self.changed.notify_all();
    }
}

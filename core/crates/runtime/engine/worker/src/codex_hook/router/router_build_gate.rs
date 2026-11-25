use super::CodexRouterError;
use std::sync::{Condvar, Mutex};

#[derive(Default)]
struct BuildState {
    outcome: Option<Result<(), CodexRouterError>>,
    #[cfg(test)]
    waiters: usize,
}

#[derive(Default)]
pub(super) struct RouteBuildGate {
    state: Mutex<BuildState>,
    changed: Condvar,
}

impl RouteBuildGate {
    pub(super) fn wait(&self) -> Result<(), CodexRouterError> {
        let mut state = self.state.lock().map_err(|_| CodexRouterError::Lock)?;
        #[cfg(test)]
        {
            state.waiters = state
                .waiters
                .checked_add(super::constants::ROUTER_LEASE_INCREMENT)
                .ok_or(CodexRouterError::Capacity)?;
            self.changed.notify_all();
        }
        while state.outcome.is_none() {
            state = self
                .changed
                .wait(state)
                .map_err(|_| CodexRouterError::Lock)?;
        }
        #[cfg(test)]
        {
            state.waiters = state
                .waiters
                .checked_sub(super::constants::ROUTER_LEASE_INCREMENT)
                .ok_or(CodexRouterError::Lock)?;
        }
        state.outcome.ok_or(CodexRouterError::Lock)?
    }

    pub(super) fn complete(&self, outcome: Result<(), CodexRouterError>) {
        let mut state = match self.state.lock() {
            Ok(state) => state,
            Err(poisoned) => poisoned.into_inner(),
        };
        if state.outcome.is_none() {
            state.outcome = Some(outcome);
        }
        self.changed.notify_all();
    }

    #[cfg(test)]
    pub(super) fn wait_for_waiters(
        &self,
        expected: usize,
        timeout: std::time::Duration,
    ) -> Result<bool, CodexRouterError> {
        let state = self.state.lock().map_err(|_| CodexRouterError::Lock)?;
        let (state, _) = self
            .changed
            .wait_timeout_while(state, timeout, |state| {
                state.waiters < expected && state.outcome.is_none()
            })
            .map_err(|_| CodexRouterError::Lock)?;
        Ok(state.waiters >= expected)
    }
}

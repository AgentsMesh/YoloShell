use super::{StartupChildGuard, WorkerState};
use anyhow::{Context, Result};

pub(crate) const FAULT_REAP_OBSERVER_ALREADY_BOUND_DETAIL: &str =
    "worker fault-reap observer is already bound to another reservation";

pub(crate) fn bind_guarded_fault_reap_observer(
    guard: &StartupChildGuard,
    state: &WorkerState,
) -> Result<()> {
    let observer = guard
        .fault_reaper
        .as_ref()
        .context(super::super::startup_guard::STARTUP_FAULT_REAPER_UNAVAILABLE_CONTEXT)?
        .state_observer();
    state.bind_fault_reap_observer(observer)
}

impl WorkerState {
    pub(crate) fn publish_owner_fault(&self, error: &anyhow::Error) {
        if let Some(observer) = self.fault_reap_observer.get() {
            observer.publish_fault(error);
        }
    }

    pub(crate) fn bind_fault_reap_observer(
        &self,
        observer: super::super::pty_fault_reaper::FaultReapStateObserver,
    ) -> Result<()> {
        match self.fault_reap_observer.set(observer) {
            Ok(()) => Ok(()),
            Err(observer)
                if self
                    .fault_reap_observer
                    .get()
                    .is_some_and(|current| current.shares_state_with(&observer)) =>
            {
                Ok(())
            }
            Err(_) => anyhow::bail!(FAULT_REAP_OBSERVER_ALREADY_BOUND_DETAIL),
        }
    }
}

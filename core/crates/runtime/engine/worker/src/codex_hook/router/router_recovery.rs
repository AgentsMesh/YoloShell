use super::state::RouterEntries;
use super::{
    CodexHookRuntime, CodexRouteEntry, CodexRouteLease, CodexRouterError, CodexSessionRouter,
    DurableCodexAdmission,
};
use crate::codex_hooks::CodexAdmissionRecord;
use std::sync::Arc;

#[must_use]
pub(in crate::worker::codex_hook) struct CodexRecoveredRoute {
    entry: Arc<CodexRouteEntry>,
    router: Arc<RouterEntries>,
    rollback: bool,
}

impl CodexSessionRouter {
    pub(in crate::worker::codex_hook) fn recover_committed<Builder, Validate>(
        &self,
        admission: DurableCodexAdmission,
        builder: Builder,
        validate: Validate,
    ) -> Result<CodexRecoveredRoute, CodexRouterError>
    where
        Builder: FnOnce(&CodexAdmissionRecord) -> Result<Arc<CodexHookRuntime>, CodexRouterError>,
        Validate: FnOnce(&CodexRouteEntry) -> Result<(), CodexRouterError>,
    {
        let lease = self.install_or_replay(admission, builder)?;
        let rollback = !lease.is_replay();
        let entry = Arc::clone(lease.entry());
        if let Err(error) = validate(&entry) {
            return Err(abort_lease(lease, error));
        }
        if let Err(error) = entry.admission().verify() {
            return Err(abort_lease(lease, error));
        }
        lease.settle()?;
        Ok(CodexRecoveredRoute::new(
            entry,
            Arc::clone(&self.inner),
            rollback,
        ))
    }
}

fn abort_lease(lease: CodexRouteLease, cause: CodexRouterError) -> CodexRouterError {
    lease.abort().err().unwrap_or(cause)
}

impl CodexRecoveredRoute {
    fn new(entry: Arc<CodexRouteEntry>, router: Arc<RouterEntries>, rollback: bool) -> Self {
        Self {
            entry,
            router,
            rollback,
        }
    }

    pub(in crate::worker::codex_hook) fn entry(&self) -> &Arc<CodexRouteEntry> {
        &self.entry
    }

    pub(in crate::worker::codex_hook) fn accept(mut self) -> Arc<CodexRouteEntry> {
        self.rollback = false;
        Arc::clone(&self.entry)
    }

    pub(in crate::worker::codex_hook) fn rollback(mut self) -> Result<(), CodexRouterError> {
        if !self.rollback {
            return Ok(());
        }
        self.router.discard_recovered(&self.entry)?;
        self.rollback = false;
        Ok(())
    }
}

impl Drop for CodexRecoveredRoute {
    fn drop(&mut self) {
        if self.rollback {
            let _ = self.router.discard_recovered(&self.entry);
        }
    }
}

impl RouterEntries {
    fn discard_recovered(&self, expected: &Arc<CodexRouteEntry>) -> Result<(), CodexRouterError> {
        let mut entries = self.lock()?;
        let Some(state) = entries.get(&expected.key()) else {
            return Ok(());
        };
        let discard = state
            .active()
            .filter(|slot| Arc::ptr_eq(slot.entry(), expected))
            .is_some_and(super::state::RouteSlot::can_discard_recovered);
        if !discard {
            return Err(CodexRouterError::Conflict);
        }
        entries.remove(&expected.key());
        Ok(())
    }
}

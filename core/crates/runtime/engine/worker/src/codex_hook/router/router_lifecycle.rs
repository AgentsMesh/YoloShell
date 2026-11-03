use super::state::{RouteState, RouterEntries};
use super::{
    CodexRetirementPermit, CodexRetirementReadiness, CodexRouteEntry, CodexRouteKey,
    CodexRouteLease, CodexRouterError, CodexSessionRouter,
};
use crate::agent::hooks::HookIdentity;
use std::sync::Arc;

impl CodexSessionRouter {
    pub(in crate::worker::codex_hook) fn resolve(
        &self,
        identity: &HookIdentity,
    ) -> Result<Arc<CodexRouteEntry>, CodexRouterError> {
        identity.validate().map_err(|_| CodexRouterError::Scope)?;
        if identity.terminal_id != self.terminal_id()
            || identity.runtime_epoch != self.runtime_epoch()
        {
            return Err(CodexRouterError::Scope);
        }
        let key = CodexRouteKey::from_identity(identity);
        let entry = self
            .inner
            .lock()?
            .get(&key)
            .and_then(RouteState::active)
            .map(|slot| Arc::clone(slot.entry()))
            .ok_or(CodexRouterError::Conflict)?;
        if &entry.admission().record().request().identity != identity {
            return Err(CodexRouterError::Conflict);
        }
        entry.admission().verify()?;
        Ok(entry)
    }

    pub(in crate::worker::codex_hook) fn begin_retirement(
        &self,
        expected: &Arc<CodexRouteEntry>,
    ) -> Result<CodexRetirementReadiness, CodexRouterError> {
        expected.admission().verify()?;
        let key = expected.key();
        let mut entries = self.inner.lock()?;
        let Some(slot) = entries
            .get_mut(&key)
            .and_then(RouteState::active_mut)
            .filter(|slot| Arc::ptr_eq(slot.entry(), expected))
        else {
            return Ok(CodexRetirementReadiness::Missing);
        };
        if slot.request_retirement()? {
            Ok(CodexRetirementReadiness::Ready(CodexRetirementPermit::new(
                Arc::clone(expected),
                Arc::clone(&self.inner),
            )))
        } else {
            Ok(CodexRetirementReadiness::Pending)
        }
    }

    pub(in crate::worker::codex_hook) fn lease_committed(
        &self,
        identity: &HookIdentity,
    ) -> Result<CodexRouteLease, CodexRouterError> {
        identity.validate().map_err(|_| CodexRouterError::Scope)?;
        if identity.terminal_id != self.terminal_id()
            || identity.runtime_epoch != self.runtime_epoch()
        {
            return Err(CodexRouterError::Scope);
        }
        let key = CodexRouteKey::from_identity(identity);
        let lease = {
            let mut entries = self.inner.lock()?;
            let slot = entries
                .get_mut(&key)
                .and_then(RouteState::active_mut)
                .ok_or(CodexRouterError::Conflict)?;
            if !slot.is_committed()
                || &slot.entry().admission().record().request().identity != identity
            {
                return Err(CodexRouterError::Conflict);
            }
            slot.acquire()?;
            CodexRouteLease::new(Arc::clone(slot.entry()), Arc::clone(&self.inner), true)
        };
        lease.entry().admission().verify()?;
        Ok(lease)
    }

    pub(in crate::worker::codex_hook) fn committed_entries(
        &self,
    ) -> Result<Vec<Arc<CodexRouteEntry>>, CodexRouterError> {
        let entries = self
            .inner
            .lock()?
            .values()
            .filter_map(RouteState::active)
            .filter(|slot| slot.is_committed())
            .map(|slot| Arc::clone(slot.entry()))
            .collect::<Vec<_>>();
        for entry in &entries {
            entry.admission().verify()?;
        }
        Ok(entries)
    }
}

impl RouterEntries {
    pub(super) fn verify_retirement_permit(
        &self,
        key: CodexRouteKey,
        expected: &Arc<CodexRouteEntry>,
    ) -> Result<(), CodexRouterError> {
        let entries = self.lock()?;
        let slot = entries
            .get(&key)
            .and_then(RouteState::active)
            .filter(|slot| Arc::ptr_eq(slot.entry(), expected))
            .ok_or(CodexRouterError::Conflict)?;
        slot.verify_retirement_permit()
    }

    pub(super) fn commit_retirement(
        &self,
        key: CodexRouteKey,
        expected: &Arc<CodexRouteEntry>,
    ) -> Result<(), CodexRouterError> {
        let mut entries = self.lock()?;
        let slot = entries
            .get(&key)
            .and_then(RouteState::active)
            .filter(|slot| Arc::ptr_eq(slot.entry(), expected))
            .ok_or(CodexRouterError::Conflict)?;
        slot.commit_retirement()?;
        entries.remove(&key);
        Ok(())
    }

    pub(super) fn release_retirement_permit(
        &self,
        key: CodexRouteKey,
        expected: &Arc<CodexRouteEntry>,
    ) -> Result<(), CodexRouterError> {
        let mut entries = self.lock()?;
        if let Some(slot) = entries
            .get_mut(&key)
            .and_then(RouteState::active_mut)
            .filter(|slot| Arc::ptr_eq(slot.entry(), expected))
        {
            slot.release_retirement_permit();
        }
        Ok(())
    }
}

impl CodexRouteLease {
    pub(in crate::worker::codex_hook) fn entry(&self) -> &Arc<CodexRouteEntry> {
        &self.entry
    }

    pub(in crate::worker::codex_hook) fn is_replay(&self) -> bool {
        self.replay
    }

    pub(in crate::worker::codex_hook) fn settle(mut self) -> Result<(), CodexRouterError> {
        match self.settle_success() {
            Ok(()) => Ok(()),
            Err(error) => self.abort_inner().and(Err(error)),
        }
    }

    pub(in crate::worker::codex_hook) fn abort(mut self) -> Result<(), CodexRouterError> {
        self.abort_inner()
    }

    fn settle_success(&mut self) -> Result<(), CodexRouterError> {
        if self.released {
            return Ok(());
        }
        self.router.settle_success(self.entry.key(), &self.entry)?;
        self.released = true;
        Ok(())
    }

    fn abort_inner(&mut self) -> Result<(), CodexRouterError> {
        if self.released {
            return Ok(());
        }
        self.router
            .abort(self.entry.key(), &self.entry, !self.replay)?;
        self.released = true;
        Ok(())
    }
}

impl Drop for CodexRouteLease {
    fn drop(&mut self) {
        let _ = self.abort_inner();
    }
}

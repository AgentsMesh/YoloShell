use super::build_gate::RouteBuildGate;
use super::constants::ROUTER_LEASE_INCREMENT;
use super::{CodexRouteEntry, CodexRouteKey, CodexRouterError, DurableCodexAdmission};
use std::collections::HashMap;
use std::sync::{Arc, Mutex, MutexGuard};

pub(super) struct RouterEntries {
    pub(super) entries: Mutex<HashMap<CodexRouteKey, RouteState>>,
    pub(super) publication_revision: Arc<std::sync::atomic::AtomicU64>,
    pub(super) publication_tx: tokio::sync::broadcast::Sender<super::CodexCommittedRoute>,
    #[cfg(test)]
    pub(super) faults: RouterFaults,
}

#[cfg(test)]
#[derive(Default)]
pub(super) struct RouterFaults {
    pub(super) settle: Mutex<Option<CodexRouterError>>,
    abort: Mutex<Option<CodexRouterError>>,
}

pub(super) enum RouteState {
    Building(RouteBuild),
    Active(RouteSlot),
}

pub(super) struct RouteBuild {
    pub(super) admission: DurableCodexAdmission,
    pub(super) gate: Arc<RouteBuildGate>,
}

pub(super) struct RouteSlot {
    entry: Arc<CodexRouteEntry>,
    pub(super) begin_leases: usize,
    pub(super) committed: bool,
    pub(super) rollback_pending: bool,
    publication_revision: Option<u64>,
    pub(super) retirement: RouteRetirementState,
    #[cfg(test)]
    automatic_retirement: bool,
}

#[derive(Clone, Copy, Eq, PartialEq)]
pub(super) enum RouteRetirementState {
    Active,
    Pending,
    Permitted,
}

impl RouterEntries {
    pub(super) fn lock(
        &self,
    ) -> Result<MutexGuard<'_, HashMap<CodexRouteKey, RouteState>>, CodexRouterError> {
        self.entries.lock().map_err(|_| CodexRouterError::Lock)
    }

    pub(super) fn abort(
        &self,
        key: CodexRouteKey,
        expected: &Arc<CodexRouteEntry>,
        inserted: bool,
    ) -> Result<(), CodexRouterError> {
        #[cfg(test)]
        if let Some(error) = self.faults.abort.lock().unwrap().take() {
            return Err(error);
        }
        let mut entries = self.lock()?;
        let Some(slot) = entries
            .get_mut(&key)
            .and_then(RouteState::active_mut)
            .filter(|slot| Arc::ptr_eq(slot.entry(), expected))
        else {
            return Ok(());
        };
        if inserted {
            slot.request_rollback();
        }
        let remove = slot.release();
        if remove {
            entries.remove(&key);
        }
        Ok(())
    }

    #[cfg(test)]
    pub(super) fn fail_next_settle(&self, error: CodexRouterError) {
        assert!(self.faults.settle.lock().unwrap().replace(error).is_none());
    }

    #[cfg(test)]
    pub(super) fn fail_next_abort(&self, error: CodexRouterError) {
        assert!(self.faults.abort.lock().unwrap().replace(error).is_none());
    }
}

impl RouteSlot {
    pub(super) fn new(entry: Arc<CodexRouteEntry>) -> Self {
        Self {
            entry,
            begin_leases: ROUTER_LEASE_INCREMENT,
            committed: false,
            rollback_pending: false,
            publication_revision: None,
            retirement: RouteRetirementState::Active,
            #[cfg(test)]
            automatic_retirement: false,
        }
    }

    pub(super) fn entry(&self) -> &Arc<CodexRouteEntry> {
        &self.entry
    }

    pub(super) fn is_committed(&self) -> bool {
        self.committed
    }

    pub(super) fn can_discard_recovered(&self) -> bool {
        self.committed
            && self.begin_leases == usize::default()
            && self.retirement != RouteRetirementState::Permitted
    }

    pub(super) fn acquire(&mut self) -> Result<(), CodexRouterError> {
        if self.retirement != RouteRetirementState::Active || self.rollback_pending {
            return Err(CodexRouterError::Conflict);
        }
        self.begin_leases = self
            .begin_leases
            .checked_add(ROUTER_LEASE_INCREMENT)
            .ok_or(CodexRouterError::Capacity)?;
        Ok(())
    }

    fn request_rollback(&mut self) {
        if !self.committed {
            self.rollback_pending = true;
        }
    }

    pub(super) fn commit(&mut self, publication_revision: u64) {
        self.committed = true;
        self.rollback_pending = false;
        self.publication_revision = Some(publication_revision);
    }

    pub(super) fn publication_revision(&self) -> Option<u64> {
        self.publication_revision
    }

    pub(super) fn release(&mut self) -> bool {
        self.begin_leases = self.begin_leases.saturating_sub(ROUTER_LEASE_INCREMENT);
        self.begin_leases == usize::default()
            && (self.rollback_pending || self.automatic_retirement_requested_for_test())
    }

    #[cfg(test)]
    pub(super) fn request_automatic_retirement_for_test(&mut self) -> bool {
        self.automatic_retirement = true;
        self.retirement = RouteRetirementState::Pending;
        self.begin_leases == usize::default()
    }

    #[cfg(test)]
    fn automatic_retirement_requested_for_test(&self) -> bool {
        self.automatic_retirement
    }

    #[cfg(not(test))]
    fn automatic_retirement_requested_for_test(&self) -> bool {
        false
    }
}

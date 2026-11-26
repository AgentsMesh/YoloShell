use super::build_gate::RouteBuildGate;
use super::constants::ROUTER_ENTRY_RESERVATION;
use super::state::{RouteBuild, RouteSlot, RouteState, RouterEntries};
use super::{CodexRouteEntry, CodexRouteKey, CodexRouterError, DurableCodexAdmission};
use std::sync::Arc;

pub(super) enum InstallDecision {
    Replay(Arc<CodexRouteEntry>),
    Build(Arc<RouteBuildGate>),
    Wait(Arc<RouteBuildGate>),
}

impl RouteState {
    pub(super) fn active(&self) -> Option<&RouteSlot> {
        match self {
            Self::Active(slot) => Some(slot),
            Self::Building(_) => None,
        }
    }

    pub(super) fn active_mut(&mut self) -> Option<&mut RouteSlot> {
        match self {
            Self::Active(slot) => Some(slot),
            Self::Building(_) => None,
        }
    }
}

impl RouteBuild {
    pub(super) fn new(admission: DurableCodexAdmission, gate: Arc<RouteBuildGate>) -> Self {
        Self { admission, gate }
    }

    pub(super) fn admission(&self) -> &DurableCodexAdmission {
        &self.admission
    }

    pub(super) fn gate(&self) -> &Arc<RouteBuildGate> {
        &self.gate
    }
}

impl RouterEntries {
    pub(super) fn prepare_install(
        &self,
        key: CodexRouteKey,
        admission: &DurableCodexAdmission,
        capacity_limit: usize,
    ) -> Result<InstallDecision, CodexRouterError> {
        let mut entries = self.lock()?;
        if let Some(state) = entries.get_mut(&key) {
            return match state {
                RouteState::Active(slot) => {
                    if !slot.entry().admission().exactly_matches(admission) {
                        return Err(CodexRouterError::Conflict);
                    }
                    slot.acquire()?;
                    Ok(InstallDecision::Replay(Arc::clone(slot.entry())))
                }
                RouteState::Building(build) => {
                    if !build.admission().exactly_matches(admission) {
                        return Err(CodexRouterError::Conflict);
                    }
                    Ok(InstallDecision::Wait(Arc::clone(build.gate())))
                }
            };
        }
        if entries.len() >= capacity_limit {
            return Err(CodexRouterError::Capacity);
        }
        entries
            .try_reserve(ROUTER_ENTRY_RESERVATION)
            .map_err(|_| CodexRouterError::Capacity)?;
        let gate = Arc::new(RouteBuildGate::default());
        entries.insert(
            key,
            RouteState::Building(RouteBuild::new(admission.clone(), Arc::clone(&gate))),
        );
        Ok(InstallDecision::Build(gate))
    }

    pub(super) fn publish_build(
        &self,
        key: CodexRouteKey,
        gate: &Arc<RouteBuildGate>,
        entry: Arc<CodexRouteEntry>,
    ) -> Result<(), CodexRouterError> {
        let mut entries = self.lock()?;
        let reserved = entries
            .get(&key)
            .and_then(|state| match state {
                RouteState::Building(build) => Some(build),
                RouteState::Active(_) => None,
            })
            .filter(|build| Arc::ptr_eq(build.gate(), gate))
            .ok_or(CodexRouterError::Conflict)?;
        if !reserved.admission().exactly_matches(entry.admission()) {
            return Err(CodexRouterError::Conflict);
        }
        entries.insert(key, RouteState::Active(RouteSlot::new(entry)));
        drop(entries);
        gate.complete(Ok(()));
        Ok(())
    }

    pub(super) fn fail_build(
        &self,
        key: CodexRouteKey,
        gate: &Arc<RouteBuildGate>,
        cause: CodexRouterError,
    ) -> CodexRouterError {
        let cleanup = self.remove_build(key, gate);
        let outcome = cleanup.err().unwrap_or(cause);
        gate.complete(Err(outcome));
        outcome
    }

    fn remove_build(
        &self,
        key: CodexRouteKey,
        gate: &Arc<RouteBuildGate>,
    ) -> Result<(), CodexRouterError> {
        let mut entries = self.lock()?;
        let matches = entries.get(&key).is_some_and(|state| match state {
            RouteState::Building(build) => Arc::ptr_eq(build.gate(), gate),
            RouteState::Active(_) => false,
        });
        if !matches {
            return Err(CodexRouterError::Conflict);
        }
        entries.remove(&key);
        Ok(())
    }

    #[cfg(test)]
    pub(super) fn build_gate(
        &self,
        key: CodexRouteKey,
    ) -> Result<Arc<RouteBuildGate>, CodexRouterError> {
        self.lock()?
            .get(&key)
            .and_then(|state| match state {
                RouteState::Building(build) => Some(Arc::clone(build.gate())),
                RouteState::Active(_) => None,
            })
            .ok_or(CodexRouterError::Conflict)
    }
}

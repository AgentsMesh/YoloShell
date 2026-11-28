use super::*;
use std::sync::atomic::Ordering;

pub(super) const RELEASE_STATE_DETAIL: &str =
    "PTY secondary-custody local release cannot commit from its current state";
pub(super) const PLATFORM_UNAVAILABLE_DETAIL: &str =
    "PTY secondary-custody platform bundle is unavailable during local release";
pub(super) const REGISTRY_UNAVAILABLE_DETAIL: &str =
    "PTY secondary-custody registry is unavailable during local release";
const SUPERSEDE_STATE_DETAIL: &str =
    "PTY secondary-custody supersede cannot commit from its current state";

#[derive(Clone, Copy, Eq, PartialEq)]
enum LocalReleasePhase {
    Active,
    Rollback,
}

impl SecondaryCustodyLease {
    pub(super) fn commit_active_release(
        self: &Arc<Self>,
        settlement: super::settlement::ExactSecondaryCustodySettlement<'_>,
    ) -> Result<super::pending_release::PendingReleaseAck, WorkerRpcError> {
        self.commit_pending(settlement, LocalReleasePhase::Active)
    }

    pub(super) fn commit_rollback_release(
        self: &Arc<Self>,
        settlement: super::settlement::ExactSecondaryCustodySettlement<'_>,
    ) -> Result<super::pending_release::PendingReleaseAck, WorkerRpcError> {
        self.commit_pending(settlement, LocalReleasePhase::Rollback)
    }

    fn commit_pending(
        self: &Arc<Self>,
        settlement: super::settlement::ExactSecondaryCustodySettlement<'_>,
        phase: LocalReleasePhase,
    ) -> Result<super::pending_release::PendingReleaseAck, WorkerRpcError> {
        settlement.validate(self)?;
        let registry = super::super::lock_recover(&self.registry)
            .as_ref()
            .and_then(Weak::upgrade)
            .ok_or_else(|| WorkerRpcError::protocol(REGISTRY_UNAVAILABLE_DETAIL))?;
        let mut entries = super::super::lock_recover(&registry.values);
        let state = super::super::lock_recover(&self.state);
        if !phase.accepts(*state) {
            return Err(WorkerRpcError::protocol(RELEASE_STATE_DETAIL));
        }
        let retirement = super::registry::prepare_exact_retirement(&mut entries, self)?;
        self.finish_commit(entries, state, retirement)
    }

    pub(super) fn commit_superseded(self: &Arc<Self>) -> Result<(), WorkerRpcError> {
        let registry = super::super::lock_recover(&self.registry)
            .as_ref()
            .and_then(Weak::upgrade)
            .ok_or_else(|| WorkerRpcError::protocol(REGISTRY_UNAVAILABLE_DETAIL))?;
        let mut entries = super::super::lock_recover(&registry.values);
        let mut state = super::super::lock_recover(&self.state);
        if !matches!(
            *state,
            SecondaryCustodyState::Activating | SecondaryCustodyState::Ready
        ) {
            return Err(WorkerRpcError::protocol(SUPERSEDE_STATE_DETAIL));
        }
        let retirement = super::registry::prepare_exact_retirement(&mut entries, self)?;
        let mut platform_slot = super::super::lock_recover(&self.platform);
        if platform_slot.is_none() {
            return Err(WorkerRpcError::protocol(PLATFORM_UNAVAILABLE_DETAIL));
        }
        let platform = platform_slot
            .take()
            .ok_or_else(|| WorkerRpcError::protocol(PLATFORM_UNAVAILABLE_DETAIL))?;
        retirement.commit_terminal(&mut entries, self, SecondaryCustodyState::Superseded);
        self.available.store(false, Ordering::Release);
        *state = SecondaryCustodyState::Superseded;
        self.publish_state(SecondaryCustodyState::Superseded);
        drop(platform_slot);
        drop(state);
        drop(entries);
        super::containment::discard_after_retirement(platform, &self.request_drain);
        Ok(())
    }

    fn finish_commit(
        self: &Arc<Self>,
        mut entries: std::sync::MutexGuard<'_, SecondaryCustodyValues>,
        mut state: std::sync::MutexGuard<'_, SecondaryCustodyState>,
        retirement: super::registry::ExactRegistryRetirement,
    ) -> Result<super::pending_release::PendingReleaseAck, WorkerRpcError> {
        let mut platform_slot = super::super::lock_recover(&self.platform);
        if platform_slot.is_none() {
            return Err(WorkerRpcError::protocol(PLATFORM_UNAVAILABLE_DETAIL));
        }
        let platform = platform_slot
            .take()
            .ok_or_else(|| WorkerRpcError::protocol(PLATFORM_UNAVAILABLE_DETAIL))?;
        retirement.commit_retained(&mut entries, self);
        self.available.store(false, Ordering::Release);
        *state = SecondaryCustodyState::ReleaseAckPending;
        self.publish_state(SecondaryCustodyState::ReleaseAckPending);
        drop(platform_slot);
        drop(state);
        drop(entries);
        let pending = super::pending_release::PendingReleaseAck::new(self);
        super::containment::discard_after_retirement(platform, &self.request_drain);
        Ok(pending)
    }
}

impl LocalReleasePhase {
    fn accepts(&self, state: SecondaryCustodyState) -> bool {
        match self {
            Self::Active => matches!(
                state,
                SecondaryCustodyState::Activating | SecondaryCustodyState::Ready
            ),
            Self::Rollback => state == SecondaryCustodyState::RollingBack,
        }
    }
}

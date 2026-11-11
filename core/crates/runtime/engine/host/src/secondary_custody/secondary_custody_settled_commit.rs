use super::*;
use std::sync::atomic::Ordering;

use super::release_commit::{
    PLATFORM_UNAVAILABLE_DETAIL, REGISTRY_UNAVAILABLE_DETAIL, RELEASE_STATE_DETAIL,
};

impl SecondaryCustodyLease {
    pub(super) fn commit_settled_release(
        self: &Arc<Self>,
        settlement: super::settlement::ExactSecondaryCustodySettlement<'_>,
    ) -> Result<(), WorkerRpcError> {
        self.commit_settled(settlement, SettledTerminal::Released)
    }

    pub(super) fn commit_settled_superseded(
        self: &Arc<Self>,
        settlement: super::settlement::ExactSecondaryCustodySettlement<'_>,
    ) -> Result<(), WorkerRpcError> {
        self.commit_settled(settlement, SettledTerminal::Superseded)
    }

    fn commit_settled(
        self: &Arc<Self>,
        settlement: super::settlement::ExactSecondaryCustodySettlement<'_>,
        terminal: SettledTerminal,
    ) -> Result<(), WorkerRpcError> {
        settlement.validate(self)?;
        #[cfg(target_os = "macos")]
        let durable_guard = (terminal == SettledTerminal::Released)
            .then(|| self.acquire_durable_retirement_participant())
            .transpose()?;
        let registry = super::super::lock_recover(&self.registry)
            .as_ref()
            .and_then(Weak::upgrade)
            .ok_or_else(|| WorkerRpcError::protocol(REGISTRY_UNAVAILABLE_DETAIL))?;
        let mut entries = super::super::lock_recover(&registry.values);
        let mut state = super::super::lock_recover(&self.state);
        if !matches!(
            *state,
            SecondaryCustodyState::Disconnected | SecondaryCustodyState::Failed
        ) {
            return Err(WorkerRpcError::protocol(RELEASE_STATE_DETAIL));
        }
        let retirement = super::registry::prepare_exact_retirement(&mut entries, self)?;
        let mut platform_slot = super::super::lock_recover(&self.platform);
        let platform_ref = platform_slot
            .as_ref()
            .ok_or_else(|| WorkerRpcError::protocol(PLATFORM_UNAVAILABLE_DETAIL))?;
        #[cfg(target_os = "macos")]
        if let Some(guard) = durable_guard.as_ref() {
            self.publish_durable_retirement(guard, platform_ref)?;
        }
        let platform = platform_slot
            .take()
            .ok_or_else(|| WorkerRpcError::protocol(PLATFORM_UNAVAILABLE_DETAIL))?;
        let terminal_state = terminal.state();
        retirement.commit_terminal(&mut entries, self, terminal_state);
        self.available.store(false, Ordering::Release);
        *state = terminal_state;
        self.publish_state(terminal_state);
        drop(platform_slot);
        drop(state);
        drop(entries);
        #[cfg(target_os = "macos")]
        drop(durable_guard);
        super::containment::discard_after_retirement(platform, &self.request_drain);
        Ok(())
    }
}

#[derive(Clone, Copy, Eq, PartialEq)]
enum SettledTerminal {
    Released,
    Superseded,
}

impl SettledTerminal {
    fn state(self) -> SecondaryCustodyState {
        match self {
            Self::Released => SecondaryCustodyState::Released,
            Self::Superseded => SecondaryCustodyState::Superseded,
        }
    }
}

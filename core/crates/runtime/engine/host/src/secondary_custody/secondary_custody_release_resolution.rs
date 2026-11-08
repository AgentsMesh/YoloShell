use super::*;

const PENDING_RELEASE_REGISTRY_DETAIL: &str =
    "PTY secondary-custody pending release registry is unavailable";
const PENDING_RELEASE_ENTRY_DETAIL: &str =
    "PTY secondary-custody pending release entry changed identity";
const PENDING_RELEASE_STATE_DETAIL: &str =
    "PTY secondary-custody pending release resolved from an impossible state";

impl SecondaryCustodyLease {
    pub(super) fn resolve_pending_release(
        self: &Arc<Self>,
        outcome: SecondaryCustodyState,
    ) -> Result<(), WorkerRpcError> {
        if !matches!(
            outcome,
            SecondaryCustodyState::Released | SecondaryCustodyState::ReleaseUncertain
        ) {
            return Err(WorkerRpcError::protocol(PENDING_RELEASE_STATE_DETAIL));
        }
        let registry = super::super::lock_recover(&self.registry)
            .as_ref()
            .and_then(Weak::upgrade)
            .ok_or_else(|| WorkerRpcError::protocol(PENDING_RELEASE_REGISTRY_DETAIL))?;
        let mut values = super::super::lock_recover(&registry.values);
        let key = SecondaryCustodyKey::from_runtime(&self.runtime);
        let retirement = values
            .retired
            .get_mut(&key)
            .ok_or_else(|| WorkerRpcError::protocol(PENDING_RELEASE_ENTRY_DETAIL))?;
        if !retirement.references(self) {
            return Err(WorkerRpcError::protocol(PENDING_RELEASE_ENTRY_DETAIL));
        }
        let mut state = super::super::lock_recover(&self.state);
        if *state != SecondaryCustodyState::ReleaseAckPending {
            return Err(WorkerRpcError::protocol(PENDING_RELEASE_STATE_DETAIL));
        }
        if outcome == SecondaryCustodyState::Released {
            retirement.replace_with_terminal(self, outcome);
            retirement::compact_terminal_retirements(&mut values, key.terminal_id);
        }
        *state = outcome;
        self.publish_state(outcome);
        Ok(())
    }
}

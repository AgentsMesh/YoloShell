use super::{
    CodexRouteEntry, CodexSessionRouter,
    state::{RouteRetirementState, RouteState},
};
use crate::agent::codex::authority::CodexBindingAuthority;
use anyhow::{Context, Result, ensure};

const UNAVAILABLE: &str = "Codex control route is busy, retired, or replaced";

impl CodexSessionRouter {
    pub(in crate::worker::codex_hook) fn with_control_route<T>(
        &self,
        authority: &CodexBindingAuthority,
        operation: impl FnOnce(&CodexRouteEntry) -> Result<T>,
    ) -> Result<T> {
        let entries = self
            .inner
            .entries
            .try_lock()
            .map_err(|_| anyhow::anyhow!(UNAVAILABLE))?;
        let slot = entries
            .iter()
            .find(|(key, _)| {
                key.runtime_epoch() == self.runtime_epoch()
                    && key.invocation_id() == authority.version.invocation_id
            })
            .and_then(|(_, state)| RouteState::active(state))
            .context(UNAVAILABLE)?;
        ensure!(
            slot.is_committed()
                && !slot.rollback_pending
                && slot.retirement == RouteRetirementState::Active
                && slot.publication_revision() == Some(authority.version.route_revision),
            UNAVAILABLE
        );
        let entry = slot.entry();
        entry.admission().verify()?;
        let record = entry.admission().record();
        let request = record.request();
        ensure!(
            record.route_revision() == authority.version.route_revision
                && request.identity.terminal_id == self.terminal_id()
                && request.identity.runtime_epoch == self.runtime_epoch()
                && request.identity.invocation_id == authority.version.invocation_id
                && request.identity.generation == authority.version.generation
                && request.identity.process == authority.provider_process(),
            UNAVAILABLE
        );
        operation(entry)
    }
}

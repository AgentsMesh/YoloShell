impl CodexHookIngressSupervisor {
    pub(in crate::worker::codex_hook) fn with_control_route<T>(
        &self,
        authority: &crate::agent::codex::authority::CodexBindingAuthority,
        operation: impl FnOnce(&super::router::CodexRouteEntry) -> anyhow::Result<T>,
    ) -> anyhow::Result<T> {
        const UNAVAILABLE: &str = "Codex control ingress is busy, restarting, or stopped";
        let _publication = self
            .inner
            .publication_gate
            .try_lock()
            .map_err(|_| anyhow::anyhow!(UNAVAILABLE))?;
        let state = self
            .inner
            .state
            .try_lock()
            .map_err(|_| anyhow::anyhow!(UNAVAILABLE))?;
        let IngressState::Running(active) = &*state else {
            anyhow::bail!(UNAVAILABLE)
        };
        anyhow::ensure!(!active.is_complete(), UNAVAILABLE);
        let _generation = active
            .control_fence()
            .ok_or_else(|| anyhow::anyhow!(UNAVAILABLE))?;
        let router = self
            .inner
            .projection_router
            .try_lock()
            .map_err(|_| anyhow::anyhow!(UNAVAILABLE))?;
        let router = router
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!(UNAVAILABLE))?;
        router.with_control_route(authority, operation)
    }
}

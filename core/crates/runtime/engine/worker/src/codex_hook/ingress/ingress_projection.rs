impl CodexHookIngressSupervisor {
    pub(super) fn track_projection(
        &self,
    ) -> anyhow::Result<super::projection_completion::ProjectionTicket> {
        self.projection_completion.start()
    }

    pub(in crate::worker) async fn wait_projection(&self) -> anyhow::Result<()> {
        self.projection_completion.wait().await
    }

    fn clear_projection_router(&self) {
        crate::worker::lock_recover(&self.inner.projection_router).take();
    }

    pub(super) fn subscribe_projection_routes(
        &self,
    ) -> anyhow::Result<super::router::CodexRouteSubscription> {
        let router = crate::worker::lock_recover(&self.inner.projection_router)
            .as_ref()
            .cloned()
            .ok_or_else(|| {
                anyhow::anyhow!(super::constants::CODEX_PROJECTION_ROUTER_UNAVAILABLE)
            })?;
        router.subscribe_committed().map_err(anyhow::Error::new)
    }
}

use crate::{
    LocalBackend,
    connector_catalogue::ObservationScope,
    native_diagnostics::{self, ObservationContext, ObservationStage},
};
use yolo_hub_agent_projection::AgentProjection;
use yolo_hub_agent_protocol::AgentSnapshot;
use yolo_hub_terminal_protocol::PublicError;

impl LocalBackend {
    /// Validate a native effect against one terminal while retaining the
    /// caller's projection lock.
    pub(crate) async fn observe_agents_locked(
        &self,
        projection: &mut AgentProjection,
        diagnostic: ObservationContext<'_>,
    ) -> Result<Vec<AgentSnapshot>, PublicError> {
        let _observation = self.connector_observation().await;
        let address = diagnostic.terminal.address;
        let terminal = address.terminal_id;
        let observed = self.client.get_session(terminal.as_uuid()).await;
        match &observed {
            Ok(view) => {
                if view.manifest.terminal_id != terminal.as_uuid() {
                    return Err(PublicError::TargetMismatch);
                }
                let native = match projection.reusable_idle_observation(view).ok().flatten() {
                    Some(observation) => Ok(observation),
                    None => self.classify_native_observation(view).await,
                };
                if view.agent.is_some()
                    && let Err(stage) = &native
                {
                    native_diagnostics::catalogue_unavailable(address, *stage);
                }
                let stage = native
                    .as_ref()
                    .err()
                    .copied()
                    .unwrap_or(ObservationStage::Coherent);
                native_diagnostics::observation(self.hub, diagnostic, stage);
                projection
                    .project_with_observation(view, native.as_ref().ok())
                    .map_err(|_| PublicError::Internal)?;
            }
            Err(_) => {
                native_diagnostics::observation(
                    self.hub,
                    diagnostic,
                    ObservationStage::HostCatalogueUnavailable,
                );
                projection
                    .mark_session_unreachable(terminal)
                    .map_err(|_| PublicError::Internal)?;
            }
        }
        let mut agents = projection.list().map_err(|_| PublicError::Internal)?;
        let views = observed.as_ref().ok().map(std::slice::from_ref);
        agents.extend(
            self.connector_catalogue(views, ObservationScope::Terminal(address))
                .await
                .agents,
        );
        Ok(agents)
    }
}

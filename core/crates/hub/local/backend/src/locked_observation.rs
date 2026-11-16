use crate::{
    LocalBackend,
    connector_catalogue::ObservationScope,
    native_diagnostics::{self, ObservationContext, ObservationStage},
};
use std::collections::BTreeSet;
use yolo_hub_agent_projection::AgentProjection;
use yolo_hub_agent_protocol::AgentSnapshot;
use yolo_hub_identity::{TerminalAddress, TerminalId};
use yolo_hub_terminal_protocol::PublicError;
use yolo_terminal_model::TerminalView;

/// The native control path admits its effect against one projection state, so
/// it keeps the projection lock across this read. Catalogue reads do not use
/// this sweep: they gather Host and Worker evidence before taking the lock.
impl LocalBackend {
    pub(crate) async fn observe_agents_locked(
        &self,
        mut projection: Option<&mut AgentProjection>,
        diagnostic: Option<ObservationContext<'_>>,
        scope: Option<TerminalId>,
    ) -> Result<Vec<AgentSnapshot>, PublicError> {
        let _observation = self.connector_observation().await;
        // Archived sessions are never observed. A scoped get/control resolves
        // only its target view; a catalogue sweep reads the live catalogue.
        let observed: Result<Vec<TerminalView>, yolo_client::ClientError> = match scope {
            Some(terminal) => self
                .client
                .get_session(terminal.as_uuid())
                .await
                .map(|view| vec![view]),
            None => self.catalogue_views(false).await,
        };
        if let Some(projection) = projection.as_deref_mut() {
            match &observed {
                Ok(views) => {
                    let mut observed = BTreeSet::new();
                    for view in views {
                        let id = TerminalId::from_uuid(view.manifest.terminal_id)
                            .map_err(|_| PublicError::Internal)?;
                        observed.insert(id);
                        // An unchanged idle binding keeps its stored witness:
                        // its readiness is the binding state itself.
                        let native =
                            match projection.reusable_idle_observation(view).ok().flatten() {
                                Some(observation) => Ok(observation),
                                None => self.classify_native_observation(view).await,
                            };
                        if view.agent.is_some()
                            && let Err(stage) = &native
                        {
                            native_diagnostics::catalogue_unavailable(
                                TerminalAddress {
                                    hub_id: self.hub,
                                    terminal_id: id,
                                },
                                *stage,
                            );
                        }
                        if let Some(context) = diagnostic.filter(|context| {
                            context.terminal.address.terminal_id.as_uuid()
                                == view.manifest.terminal_id
                        }) {
                            let stage = native
                                .as_ref()
                                .err()
                                .copied()
                                .unwrap_or(ObservationStage::Coherent);
                            native_diagnostics::observation(self.hub, context, stage);
                        }
                        let native = native.ok();
                        projection
                            .project_with_observation(view, native.as_ref())
                            .map_err(|_| PublicError::Internal)?;
                    }
                    if scope.is_none() {
                        projection
                            .mark_missing_sessions(&observed)
                            .map_err(|_| PublicError::Internal)?;
                    }
                }
                Err(_) => {
                    if let Some(context) = diagnostic {
                        native_diagnostics::observation(
                            self.hub,
                            context,
                            ObservationStage::HostCatalogueUnavailable,
                        );
                    }
                    let marked = match scope {
                        Some(terminal) => projection.mark_session_unreachable(terminal),
                        None => projection.mark_unreachable(),
                    };
                    marked.map_err(|_| PublicError::Internal)?;
                }
            }
        }
        let mut agents = match projection.as_deref() {
            Some(projection) => projection.list().map_err(|_| PublicError::Internal)?,
            None => Vec::new(),
        };
        let scope = match scope {
            Some(terminal) => ObservationScope::Terminal(TerminalAddress {
                hub_id: self.hub,
                terminal_id: terminal,
            }),
            None => ObservationScope::Complete,
        };
        let connectors = self
            .connector_catalogue(observed.as_deref().ok(), scope)
            .await;
        agents.extend(connectors.agents);
        Ok(agents)
    }
}

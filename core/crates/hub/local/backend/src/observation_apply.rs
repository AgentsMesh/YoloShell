use crate::{
    LocalBackend, native_diagnostics,
    observation::{AppliedCatalogue, ObservationSweep, SweepPolicy},
};
use std::collections::BTreeSet;
use yolo_hub_identity::{TerminalAddress, TerminalId};
use yolo_hub_terminal_protocol::PublicError;

impl LocalBackend {
    /// Give an observed provider execution a durable run identity when it has
    /// none. The platform service decides which observations qualify.
    pub(crate) async fn adopt_observed(
        &self,
        agents: &[yolo_hub_agent_protocol::AgentSnapshot],
    ) {
        let runs = self.platform.as_ref().map(|platform| platform.runs());
        yolo_hub_local_platform::adopt_observed(runs, agents).await;
    }

    /// Applies a gathered sweep under the projection lock. Everything here is
    /// local record work: no Host or Worker call runs while the lock is held.
    pub(crate) async fn apply_catalogue(
        &self,
        sweep: ObservationSweep,
        project_archived: bool,
        policy: SweepPolicy,
    ) -> Result<AppliedCatalogue, PublicError> {
        let ObservationSweep {
            views,
            native,
            connectors,
        } = sweep;
        let mut projected = Vec::new();
        let stored = match &self.agents {
            Some(projection) => {
                let mut projection = projection.lock().await;
                match &views {
                    Ok(views) => {
                        let mut observed = BTreeSet::new();
                        for (view, native) in views.iter().zip(native) {
                            let id = TerminalId::from_uuid(view.manifest.terminal_id)
                                .map_err(|_| PublicError::Internal)?;
                            observed.insert(id);
                            if view.agent.is_some()
                                && !view.manifest.archived
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
                            let snapshot = if view.manifest.archived && !project_archived {
                                None
                            } else {
                                projection
                                    .project_with_observation(view, native.as_ref().ok())
                                    .map_err(|_| PublicError::Internal)?
                            };
                            projected.push(snapshot);
                        }
                        if matches!(policy, SweepPolicy::ObserveAgents(None)) {
                            projection
                                .mark_missing_sessions(&observed)
                                .map_err(|_| PublicError::Internal)?;
                        }
                    }
                    Err(_) => {
                        if let SweepPolicy::ObserveAgents(scope) = policy {
                            let marked = match scope {
                                Some(terminal) => projection.mark_session_unreachable(terminal),
                                None => projection.mark_unreachable(),
                            };
                            marked.map_err(|_| PublicError::Internal)?;
                        }
                    }
                }
                projection.list().map_err(|_| PublicError::Internal)?
            }
            None => {
                if let Ok(views) = &views {
                    projected = vec![None; views.len()];
                }
                Vec::new()
            }
        };
        let mut agents = stored;
        agents.extend(connectors.agents.iter().cloned());
        Ok(AppliedCatalogue {
            views,
            projected,
            agents,
            connectors,
        })
    }
}

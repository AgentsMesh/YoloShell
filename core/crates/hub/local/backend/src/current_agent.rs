use crate::LocalBackend;
use std::collections::BTreeSet;
use yolo_hub_identity::{AgentAddress, RuntimeEpoch, RuntimeTarget, TerminalAddress, TerminalId};
use yolo_hub_rpc_protocol::Reply;
use yolo_hub_terminal_protocol::PublicError;

impl LocalBackend {
    pub(crate) async fn get_current_agent(
        &self,
        address: AgentAddress,
    ) -> Result<Reply, PublicError> {
        if self.agents.is_none() && self.connectors.is_none() {
            return Err(PublicError::Unsupported);
        }
        let applied = self
            .observe_applied_catalogue(
                crate::connector_catalogue::ObservationScope::Complete,
                false,
                true,
                crate::observation::SweepPolicy::ObserveAgents(None),
            )
            .await?;
        let crate::observation::AppliedCatalogue {
            views,
            agents,
            ..
        } = applied;
        self.adopt_observed(&agents).await;
        let sessions = views.map_err(crate::errors::map_error)?;
        let current_targets = sessions
            .iter()
            .filter_map(|view| {
                let runtime = view.runtime.as_ref()?;
                if !matches!(
                    runtime.status,
                    yolo_terminal_model::TerminalRuntimeStatus::Starting
                        | yolo_terminal_model::TerminalRuntimeStatus::Running
                        | yolo_terminal_model::TerminalRuntimeStatus::Finalizing
                        | yolo_terminal_model::TerminalRuntimeStatus::OutputBlocked
                ) {
                    return None;
                }
                Some(RuntimeTarget {
                    address: TerminalAddress {
                        hub_id: self.hub,
                        terminal_id: TerminalId::from_uuid(view.manifest.terminal_id).ok()?,
                    },
                    runtime_epoch: RuntimeEpoch::from_uuid(runtime.runtime_epoch).ok()?,
                })
            })
            .collect::<BTreeSet<_>>();
        let candidates = agents
            .into_iter()
            .filter(|agent| {
                agent.cursor().target().address == address
                    && current_targets.contains(&agent.fields().terminal)
                    && matches!(
                        agent.fields().lifecycle,
                        yolo_hub_agent_protocol::Lifecycle::Starting {}
                            | yolo_hub_agent_protocol::Lifecycle::Running {}
                            | yolo_hub_agent_protocol::Lifecycle::Unknown {}
                    )
                    && agent.fields().freshness == yolo_hub_agent_protocol::Freshness::Current
                    && agent.fields().reachability == yolo_hub_agent_protocol::Reachability::Reachable
            })
            .collect::<Vec<_>>();
        match candidates.as_slice() {
            [agent] => Ok(Reply::Agent {
                agent: agent.clone(),
            }),
            [] => Err(PublicError::NotFound),
            _ => Err(PublicError::Conflict),
        }
    }
}

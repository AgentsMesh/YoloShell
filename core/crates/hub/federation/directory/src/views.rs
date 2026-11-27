use crate::OriginBook;
use anyhow::Result;
use yolo_hub_agent_protocol::{AgentSnapshot, Freshness, Reachability};
use yolo_hub_identity::HubId;
use yolo_hub_peer_protocol::Catalogue;
use yolo_hub_routing::{Right, Rights};
use yolo_hub_rpc_protocol::{HubSummary, RouteCapabilities};

impl OriginBook {
    pub fn route_capabilities(
        &self,
        local: HubId,
        target: HubId,
        now: u64,
    ) -> Result<RouteCapabilities> {
        if target == local {
            return Ok(RouteCapabilities {
                lifecycle: true,
                terminal_read: true,
                terminal_write: true,
                agent_start: true,
            });
        }
        let lifecycle: Rights = [Right::Lifecycle].into_iter().collect();
        let terminal_read: Rights = [Right::TerminalRead].into_iter().collect();
        let terminal_write: Rights = [Right::TerminalWrite].into_iter().collect();
        let agent_start: Rights = [Right::AgentControl, Right::Lifecycle, Right::TerminalWrite]
            .into_iter()
            .collect();
        Ok(RouteCapabilities {
            lifecycle: self.route_for(target, &lifecycle, now)?.is_some(),
            terminal_read: self.route_for(target, &terminal_read, now)?.is_some(),
            terminal_write: self.route_for(target, &terminal_write, now)?.is_some(),
            agent_start: self.route_for(target, &agent_start, now)?.is_some(),
        })
    }
}

impl OriginBook {
    /// Only authenticated catalogues establish known origins; configuration pins do not.
    pub fn hubs(&self, now: u64) -> Result<Vec<HubSummary>> {
        let routes = self.build_routes(&self.proofs, &self.latest, now)?;
        let observe = [Right::Observe].into_iter().collect();
        Ok(self
            .latest
            .keys()
            .map(|hub_id| HubSummary {
                hub_id: *hub_id,
                reachable: routes.route_for(*hub_id, &observe).is_some(),
                label: None,
            })
            .collect())
    }

    pub fn observations(&self, now: u64) -> Result<Vec<Catalogue>> {
        let routes = self.build_routes(&self.proofs, &self.latest, now)?;
        self.latest
            .iter()
            .map(|(origin, exported)| {
                let mut catalogue = exported.payload.clone();
                let reachable = routes
                    .route_for(*origin, &[Right::Observe].into_iter().collect())
                    .is_some();
                if !reachable || !catalogue.is_current(now) {
                    for session in &mut catalogue.terminals {
                        session.mark_unreachable();
                    }
                    for agent in &mut catalogue.agents {
                        let mut fields = agent.fields().clone();
                        fields.freshness = Freshness::Stale;
                        fields.reachability = Reachability::Unreachable;
                        *agent = AgentSnapshot::new(fields)?;
                    }
                }
                Ok(catalogue)
            })
            .collect()
    }
}

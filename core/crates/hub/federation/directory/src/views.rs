use crate::OriginBook;
use anyhow::Result;
use yolo_hub_agent_protocol::{AgentSnapshot, Freshness, Reachability};
use yolo_hub_peer_protocol::Catalogue;
use yolo_hub_routing::Right;
use yolo_hub_rpc_protocol::HubSummary;

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

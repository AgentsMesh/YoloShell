use yolo_hub_identity::HubId;
use yolo_hub_rpc_protocol::{HubSummary, Reply};

/// Origin-dispatched queries disclose only this Hub, never its peer topology.
pub fn list_hubs(hub: HubId, after: Option<HubId>) -> Reply {
    let hubs = after
        .is_none_or(|after| hub > after)
        .then_some(HubSummary {
            hub_id: hub,
            reachable: true,
            label: None,
        })
        .into_iter()
        .collect();
    Reply::Hubs { hubs, next: None }
}

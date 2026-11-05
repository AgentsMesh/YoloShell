use std::collections::BTreeMap;
use yolo_hub_federation_directory::OriginBook;
use yolo_hub_federation_settings::Configuration;
use yolo_hub_identity::HubId;
use yolo_hub_rpc_protocol::{HubSummary, MAX_CATALOG_PAGE_HUBS, Reply};
use yolo_hub_terminal_protocol::PublicError;

pub(crate) fn list_hubs(
    local: HubId,
    book: &OriginBook,
    now: u64,
    after: Option<HubId>,
    labels: &BTreeMap<HubId, String>,
) -> Result<Reply, PublicError> {
    let remote = book.hubs(now).map_err(|_| PublicError::Internal)?;
    Ok(page(local, remote, after, labels))
}

/// Machine labels come from the entry Hub's configured peer dials; an
/// unconfigured peer keeps its opaque Hub id.
pub(crate) fn labels(config: &Configuration) -> BTreeMap<HubId, String> {
    config
        .peers
        .iter()
        .filter_map(|peer| {
            let dial = peer.dial.as_ref()?;
            Some((
                peer.identity.hub(),
                dial.ssh_target()
                    .or_else(|| dial.profile_id())
                    .unwrap_or_default()
                    .to_owned(),
            ))
        })
        .collect()
}

fn page(
    local: HubId,
    remote: Vec<HubSummary>,
    after: Option<HubId>,
    labels: &BTreeMap<HubId, String>,
) -> Reply {
    let mut known = BTreeMap::from([(local, true)]);
    for hub in remote {
        let reachable = known.entry(hub.hub_id).or_default();
        *reachable |= hub.reachable;
    }
    let mut hubs: Vec<_> = known
        .into_iter()
        .filter(|(hub, _)| after.is_none_or(|after| *hub > after))
        .map(|(hub_id, reachable)| HubSummary { hub_id, reachable, label: labels.get(&hub_id).cloned() })
        .collect();
    let more = hubs.len() > MAX_CATALOG_PAGE_HUBS;
    hubs.truncate(MAX_CATALOG_PAGE_HUBS);
    let next = more.then(|| hubs.last().map(|hub| hub.hub_id)).flatten();
    Reply::Hubs { hubs, next }
}

#[cfg(all(test, feature = "yolo-unit-tests"))]
#[path = "../unit_tests/hub_inventory.rs"]
mod unit_tests;

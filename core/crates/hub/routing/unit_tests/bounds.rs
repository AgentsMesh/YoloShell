use crate::support::{constants::*, *};
use yolo_hub_routing::{Catalog, RoutingError};

#[test]
fn peer_and_origin_limits_allow_updates_without_allowing_growth() {
    let mut bounded = limits();
    bounded.max_peers = limit(SINGLE_ITEM);
    bounded.max_origins_per_peer = limit(SINGLE_ITEM);
    let mut local = Catalog::new(hub(HUB_A), bounded);
    local.authorize(hub(HUB_B), hub(HUB_B), grant()).unwrap();
    local.authorize(hub(HUB_B), hub(HUB_B), grant()).unwrap();
    assert_eq!(
        local.authorize(hub(HUB_C), hub(HUB_C), grant()),
        Err(RoutingError::PeerLimit)
    );
    assert_eq!(
        local.authorize(hub(HUB_B), hub(HUB_C), grant()),
        Err(RoutingError::OriginLimit)
    );
    assert!(local.revoke(hub(HUB_B), hub(HUB_B)));
    local.authorize(hub(HUB_C), hub(HUB_C), grant()).unwrap();
}

#[test]
fn agent_limit_counts_offline_identities_until_explicitly_forgotten() {
    let mut bounded = limits();
    bounded.max_known_agents = limit(SINGLE_ITEM);
    let mut local = Catalog::new(hub(HUB_A), bounded);
    let target = local.register_local(agent(AGENT_ONE), grant()).unwrap();
    local.register_local(agent(AGENT_ONE), grant()).unwrap();
    assert_eq!(
        local.register_local(agent(AGENT_TWO), grant()),
        Err(RoutingError::AgentLimit)
    );
    let connection = connected(&mut local, HUB_B, &[HUB_B]);
    let incoming = advertisement(HUB_B, AGENT_ONE, &[HUB_B]);
    assert_eq!(
        local.replace_from(connection, FIRST_REVISION, vec![incoming.clone()]),
        Err(RoutingError::AgentLimit)
    );
    local.unregister_local(target.agent_id);
    assert_eq!(
        local.replace_from(connection, FIRST_REVISION, vec![incoming.clone()]),
        Err(RoutingError::AgentLimit)
    );
    assert!(local.forget_offline(target));
    local
        .replace_from(connection, FIRST_REVISION, vec![incoming.clone()])
        .unwrap();
    local
        .replace_from(connection, SECOND_REVISION, vec![incoming])
        .unwrap();
}

#[test]
fn route_limit_applies_across_peers_but_replacement_reuses_capacity() {
    let mut bounded = limits();
    bounded.max_routes_per_agent = limit(SINGLE_ITEM);
    let mut local = Catalog::new(hub(HUB_A), bounded);
    let via_b = connected(&mut local, HUB_B, &[HUB_C]);
    let via_c = connected(&mut local, HUB_C, &[HUB_C]);
    let b_route = advertisement(HUB_C, AGENT_ONE, &[HUB_B, HUB_C]);
    let longer = advertisement(HUB_C, AGENT_ONE, &[HUB_B, HUB_D, HUB_C]);
    assert_eq!(
        local.replace_from(via_b, FIRST_REVISION, vec![b_route.clone(), longer]),
        Err(RoutingError::RouteLimit)
    );
    local
        .replace_from(via_b, FIRST_REVISION, vec![b_route.clone()])
        .unwrap();
    local
        .replace_from(via_b, SECOND_REVISION, vec![b_route])
        .unwrap();
    let direct = advertisement(HUB_C, AGENT_ONE, &[HUB_C]);
    assert_eq!(
        local.replace_from(via_c, FIRST_REVISION, vec![direct.clone()]),
        Err(RoutingError::RouteLimit)
    );
    local.disconnect(via_b).unwrap();
    local
        .replace_from(via_c, FIRST_REVISION, vec![direct])
        .unwrap();
}

#[test]
fn incoming_and_outgoing_advertisements_are_bounded_without_partial_output() {
    let mut bounded = limits();
    bounded.max_advertisements_per_peer = limit(SINGLE_ITEM);
    let mut local = Catalog::new(hub(HUB_A), bounded);
    let connection = connected(&mut local, HUB_B, &[HUB_B]);
    let one = advertisement(HUB_B, AGENT_ONE, &[HUB_B]);
    let two = advertisement(HUB_B, AGENT_TWO, &[HUB_B]);
    assert_eq!(
        local.replace_from(connection, FIRST_REVISION, vec![one.clone(), two]),
        Err(RoutingError::AdvertisementLimit)
    );
    assert!(local.entries().is_empty());
    local
        .replace_from(connection, FIRST_REVISION, vec![one])
        .unwrap();
    local.register_local(agent(AGENT_ONE), grant()).unwrap();
    local.register_local(agent(AGENT_TWO), grant()).unwrap();
    assert_eq!(
        local.advertisements_for(hub(HUB_C), &policy(&[HUB_A])),
        Err(RoutingError::AdvertisementLimit)
    );
    local.unregister_local(agent(AGENT_TWO));
    assert_eq!(
        local
            .advertisements_for(hub(HUB_C), &policy(&[HUB_A]))
            .unwrap()
            .len(),
        SINGLE_ITEM
    );
}

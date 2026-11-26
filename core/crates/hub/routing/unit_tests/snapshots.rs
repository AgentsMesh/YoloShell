use crate::support::{constants::*, *};
use yolo_hub_routing::{Grant, Right, RoutingError};

#[test]
fn complete_snapshots_replace_peer_paths_and_reject_replays() {
    let mut local = catalog(HUB_A);
    let connection = connected(&mut local, HUB_B, &[HUB_B]);
    let resource = advertisement(HUB_B, AGENT_ONE, &[HUB_B]);
    assert_eq!(
        local.replace_from(connection, INVALID_REVISION, vec![resource.clone()]),
        Err(RoutingError::StaleRevision)
    );
    local
        .replace_from(connection, FIRST_REVISION, vec![resource])
        .unwrap();
    assert_eq!(
        local.replace_from(connection, FIRST_REVISION, Vec::new()),
        Err(RoutingError::StaleRevision)
    );
    assert!(local.entries().first().unwrap().reachable);
    local
        .replace_from(connection, SECOND_REVISION, Vec::new())
        .unwrap();
    assert!(!local.entries().first().unwrap().reachable);
}

#[test]
fn receive_attenuates_claims_and_rejects_disjoint_rights_atomically() {
    let mut local = catalog(HUB_A);
    let ceiling = Grant {
        rights: observe(),
        remaining_reexports: SINGLE_REEXPORT,
    };
    local
        .authorize(hub(HUB_B), hub(HUB_B), ceiling.clone())
        .unwrap();
    let connection = local.connect(hub(HUB_B)).unwrap();
    let resource = advertisement(HUB_B, AGENT_ONE, &[HUB_B]);
    local
        .replace_from(connection, FIRST_REVISION, vec![resource.clone()])
        .unwrap();
    assert_eq!(
        local.route_for(resource.address, &observe()).unwrap().grant,
        ceiling
    );
    let mut denied = advertisement(HUB_B, AGENT_TWO, &[HUB_B]);
    denied.grant.rights = [Right::FileRead].into_iter().collect();
    assert_eq!(
        local.replace_from(connection, SECOND_REVISION, vec![resource.clone(), denied]),
        Err(RoutingError::EmptyRights)
    );
    assert_eq!(local.entries().len(), SINGLE_ITEM);
    assert!(local.route_for(resource.address, &observe()).is_some());
}

#[test]
fn changing_authorization_invalidates_previous_advertisements() {
    let mut local = catalog(HUB_A);
    let connection = connected(&mut local, HUB_B, &[HUB_B]);
    let resource = advertisement(HUB_B, AGENT_ONE, &[HUB_B]);
    local
        .replace_from(connection, FIRST_REVISION, vec![resource.clone()])
        .unwrap();
    let ceiling = Grant {
        rights: observe(),
        remaining_reexports: NO_REEXPORTS,
    };
    local.authorize(hub(HUB_B), hub(HUB_B), ceiling).unwrap();
    assert!(local.route_for(resource.address, &observe()).is_none());
    local
        .replace_from(connection, SECOND_REVISION, vec![resource.clone()])
        .unwrap();
    assert!(local.route_for(resource.address, &observe()).is_some());
    assert!(local.route_for(resource.address, &rights()).is_none());
}

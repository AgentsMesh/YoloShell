use crate::support::{constants::*, *};
use std::collections::BTreeMap;
use yolo_hub_routing::{Catalog, Grant, Right, RoutingError};

#[test]
fn local_publication_requires_recipient_policy_but_no_reexport_budget() {
    let mut local = catalog(HUB_A);
    let mut local_grant = grant();
    local_grant.remaining_reexports = NO_REEXPORTS;
    let target = local.register_local(agent(AGENT_ONE), local_grant).unwrap();
    assert!(
        local
            .advertisements_for(hub(HUB_B), &BTreeMap::new())
            .unwrap()
            .is_empty()
    );
    assert_eq!(
        local.advertisements_for(hub(HUB_A), &policy(&[HUB_A])),
        Err(RoutingError::LocalPeer)
    );
    let exported = local
        .advertisements_for(hub(HUB_B), &policy(&[HUB_A]))
        .unwrap();
    assert_eq!(exported.len(), SINGLE_ITEM);
    let only = exported.first().unwrap();
    assert_eq!(only.address, target);
    assert_eq!(only.path, vec![hub(HUB_A)]);
    assert_eq!(only.grant.remaining_reexports, NO_REEXPORTS);
}

#[test]
fn transit_export_consumes_budget_and_never_exports_toward_a_path_member() {
    let mut local = catalog(HUB_B);
    let connection = connected(&mut local, HUB_C, &[HUB_C]);
    let mut transit = advertisement(HUB_C, AGENT_ONE, &[HUB_C]);
    transit.grant.remaining_reexports = SINGLE_REEXPORT;
    local
        .replace_from(connection, FIRST_REVISION, vec![transit.clone()])
        .unwrap();
    assert!(
        local
            .advertisements_for(hub(HUB_C), &policy(&[HUB_C]))
            .unwrap()
            .is_empty()
    );
    let exported = local
        .advertisements_for(hub(HUB_A), &policy(&[HUB_C]))
        .unwrap();
    assert_eq!(
        exported.first().unwrap().grant.remaining_reexports,
        NO_REEXPORTS
    );
    assert_eq!(exported.first().unwrap().path, vec![hub(HUB_B), hub(HUB_C)]);
    transit.grant.remaining_reexports = NO_REEXPORTS;
    local
        .replace_from(connection, SECOND_REVISION, vec![transit])
        .unwrap();
    assert!(
        local
            .advertisements_for(hub(HUB_A), &policy(&[HUB_C]))
            .unwrap()
            .is_empty()
    );
}

#[test]
fn export_rights_and_remaining_budget_are_attenuated_again() {
    let mut local = catalog(HUB_A);
    local.register_local(agent(AGENT_ONE), grant()).unwrap();
    let ceiling = Grant {
        rights: observe(),
        remaining_reexports: SINGLE_REEXPORT,
    };
    let restricted = BTreeMap::from([(hub(HUB_A), ceiling.clone())]);
    assert_eq!(
        local
            .advertisements_for(hub(HUB_B), &restricted)
            .unwrap()
            .first()
            .unwrap()
            .grant,
        ceiling
    );
    local.register_local(agent(AGENT_ONE), ceiling).unwrap();
    let denied = Grant {
        rights: [Right::FileRead].into_iter().collect(),
        remaining_reexports: REEXPORTS,
    };
    assert!(
        local
            .advertisements_for(hub(HUB_B), &BTreeMap::from([(hub(HUB_A), denied)]))
            .unwrap()
            .is_empty()
    );
}

#[test]
fn max_path_cannot_grow_on_export_and_offline_resources_are_not_exported() {
    let mut bounded = limits();
    bounded.max_path_hubs = limit(TWO_ITEMS);
    let mut local = Catalog::new(hub(HUB_B), bounded);
    let connection = connected(&mut local, HUB_C, &[HUB_C]);
    local
        .replace_from(
            connection,
            FIRST_REVISION,
            vec![advertisement(HUB_C, AGENT_ONE, &[HUB_C])],
        )
        .unwrap();
    assert!(
        local
            .advertisements_for(hub(HUB_A), &policy(&[HUB_C]))
            .unwrap()
            .is_empty()
    );
    local.disconnect(connection).unwrap();
    assert!(
        local
            .advertisements_for(hub(HUB_A), &policy(&[HUB_C]))
            .unwrap()
            .is_empty()
    );
}

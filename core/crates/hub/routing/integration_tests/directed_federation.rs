use crate::support::{constants::*, *};
use std::collections::BTreeSet;
use yolo_hub_routing::{Grant, RoutingError};

#[test]
fn each_entry_hub_has_its_own_directed_aggregate_not_a_global_graph() {
    let mut a = catalog(HUB_A);
    let mut b = catalog(HUB_B);
    let mut c = catalog(HUB_C);
    a.register_local(agent(AGENT_ONE), grant()).unwrap();
    b.register_local(agent(AGENT_TWO), grant()).unwrap();
    let c_agent = c.register_local(agent(AGENT_THREE), grant()).unwrap();
    let b_to_c = connected(&mut b, HUB_C, &[HUB_C]);
    let from_c = c.advertisements_for(hub(HUB_B), &policy(&[HUB_C])).unwrap();
    b.replace_from(b_to_c, FIRST_REVISION, from_c).unwrap();
    let a_to_b = connected(&mut a, HUB_B, &[HUB_B, HUB_C]);
    let from_b = b
        .advertisements_for(hub(HUB_A), &policy(&[HUB_B, HUB_C]))
        .unwrap();
    a.replace_from(a_to_b, FIRST_REVISION, from_b).unwrap();
    assert_eq!(
        a.reachable_origins(),
        BTreeSet::from([hub(HUB_A), hub(HUB_B), hub(HUB_C)])
    );
    assert_eq!(
        b.reachable_origins(),
        BTreeSet::from([hub(HUB_B), hub(HUB_C)])
    );
    assert_eq!(c.reachable_origins(), BTreeSet::from([hub(HUB_C)]));
    assert_eq!(a.entries().len(), THREE_ITEMS);
    assert_eq!(b.entries().len(), TWO_ITEMS);
    assert_eq!(c.entries().len(), SINGLE_ITEM);
    assert_eq!(
        a.route_for(c_agent, &rights()).unwrap().path,
        vec![hub(HUB_A), hub(HUB_B), hub(HUB_C)]
    );
    assert_eq!(
        b.route_for(c_agent, &rights()).unwrap().path,
        vec![hub(HUB_B), hub(HUB_C)]
    );
    assert_eq!(
        c.route_for(c_agent, &rights()).unwrap().path,
        vec![hub(HUB_C)]
    );
}

#[test]
fn origin_delegation_can_allow_one_relay_without_allowing_indefinite_sharing() {
    let mut a = catalog(HUB_A);
    let mut b = catalog(HUB_B);
    let mut c = catalog(HUB_C);
    let delegated = Grant {
        rights: observe(),
        remaining_reexports: SINGLE_REEXPORT,
    };
    let target = c.register_local(agent(AGENT_ONE), delegated).unwrap();
    let bc = connected(&mut b, HUB_C, &[HUB_C]);
    b.replace_from(
        bc,
        FIRST_REVISION,
        c.advertisements_for(hub(HUB_B), &policy(&[HUB_C])).unwrap(),
    )
    .unwrap();
    let ab = connected(&mut a, HUB_B, &[HUB_C]);
    a.replace_from(
        ab,
        FIRST_REVISION,
        b.advertisements_for(hub(HUB_A), &policy(&[HUB_C])).unwrap(),
    )
    .unwrap();
    assert!(a.route_for(target, &observe()).is_some());
    assert!(a.route_for(target, &rights()).is_none());
    assert!(
        a.advertisements_for(hub(HUB_D), &policy(&[HUB_C]))
            .unwrap()
            .is_empty()
    );
}

#[test]
fn direct_peer_trust_is_not_transitive_and_cycles_are_not_reexported() {
    let mut a = catalog(HUB_A);
    let mut b = catalog(HUB_B);
    let mut c = catalog(HUB_C);
    let target = c.register_local(agent(AGENT_ONE), grant()).unwrap();
    let bc = connected(&mut b, HUB_C, &[HUB_C]);
    b.replace_from(
        bc,
        FIRST_REVISION,
        c.advertisements_for(hub(HUB_B), &policy(&[HUB_C])).unwrap(),
    )
    .unwrap();
    let ab = connected(&mut a, HUB_B, &[HUB_B]);
    let transit = b.advertisements_for(hub(HUB_A), &policy(&[HUB_C])).unwrap();
    assert_eq!(
        a.replace_from(ab, FIRST_REVISION, transit.clone()),
        Err(RoutingError::UnauthorizedOrigin)
    );
    a.authorize(hub(HUB_B), hub(HUB_C), grant()).unwrap();
    a.replace_from(ab, FIRST_REVISION, transit).unwrap();
    assert!(a.route_for(target, &observe()).is_some());
    assert!(
        a.advertisements_for(hub(HUB_C), &policy(&[HUB_C]))
            .unwrap()
            .is_empty()
    );
}

use crate::support::{constants::*, *};

#[test]
fn relay_disconnect_preserves_identity_and_alternate_path_without_execution_transfer() {
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
    let ab = connected(&mut a, HUB_B, &[HUB_C]);
    a.replace_from(
        ab,
        FIRST_REVISION,
        b.advertisements_for(hub(HUB_A), &policy(&[HUB_C])).unwrap(),
    )
    .unwrap();
    let ac = connected(&mut a, HUB_C, &[HUB_C]);
    a.replace_from(
        ac,
        FIRST_REVISION,
        c.advertisements_for(hub(HUB_A), &policy(&[HUB_C])).unwrap(),
    )
    .unwrap();
    assert_eq!(a.entries().len(), SINGLE_ITEM);
    a.disconnect(ab).unwrap();
    assert_eq!(
        a.route_for(target, &observe()).unwrap().next_hop(),
        Some(hub(HUB_C))
    );
    a.disconnect(ac).unwrap();
    assert!(!a.entries().first().unwrap().reachable);
    assert_eq!(a.entries().first().unwrap().address, target);
    assert!(c.route_for(target, &rights()).is_some());
    let fresh = a.connect(hub(HUB_B)).unwrap();
    a.replace_from(
        fresh,
        FIRST_REVISION,
        b.advertisements_for(hub(HUB_A), &policy(&[HUB_C])).unwrap(),
    )
    .unwrap();
    assert_eq!(a.entries().len(), SINGLE_ITEM);
    assert_eq!(
        a.route_for(target, &observe()).unwrap().next_hop(),
        Some(hub(HUB_B))
    );
}

#[test]
fn origin_revocation_propagates_as_missing_routes_without_deleting_known_identity() {
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
    let ab = connected(&mut a, HUB_B, &[HUB_C]);
    a.replace_from(
        ab,
        FIRST_REVISION,
        b.advertisements_for(hub(HUB_A), &policy(&[HUB_C])).unwrap(),
    )
    .unwrap();
    b.revoke(hub(HUB_C), hub(HUB_C));
    a.replace_from(
        ab,
        SECOND_REVISION,
        b.advertisements_for(hub(HUB_A), &policy(&[HUB_C])).unwrap(),
    )
    .unwrap();
    assert!(a.route_for(target, &observe()).is_none());
    assert_eq!(a.entries().first().unwrap().address, target);
    assert!(c.route_for(target, &rights()).is_some());
}

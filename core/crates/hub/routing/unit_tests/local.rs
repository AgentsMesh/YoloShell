use crate::support::{constants::*, *};
use yolo_hub_routing::{Grant, Rights, RoutingError};

#[test]
fn local_identity_survives_unregister_and_explicit_forget() {
    let mut local = catalog(HUB_A);
    assert_eq!(local.local_hub(), hub(HUB_A));
    assert!(local.entries().is_empty());
    assert!(local.reachable_origins().is_empty());
    let target = local.register_local(agent(AGENT_ONE), grant()).unwrap();
    assert_eq!(target, address(HUB_A, AGENT_ONE));
    let route = local.route_for(target, &rights()).unwrap();
    assert_eq!(route.path, vec![hub(HUB_A)]);
    assert_eq!(route.next_hop(), None);
    assert!(!local.forget_offline(target));
    assert!(local.unregister_local(target.agent_id));
    assert!(!local.entries().first().unwrap().reachable);
    assert!(!local.unregister_local(target.agent_id));
    assert!(!local.unregister_local(agent(AGENT_TWO)));
    assert!(local.route_for(target, &observe()).is_none());
    assert!(local.forget_offline(target));
    assert!(!local.forget_offline(target));
    assert_eq!(
        local.register_local(target.agent_id, grant()).unwrap(),
        target
    );
}

#[test]
fn local_publication_replaces_permissions_without_duplicate_identity() {
    let mut local = catalog(HUB_A);
    let target = local.register_local(agent(AGENT_ONE), grant()).unwrap();
    let restricted = Grant {
        rights: observe(),
        remaining_reexports: NO_REEXPORTS,
    };
    assert_eq!(
        local
            .register_local(target.agent_id, restricted.clone())
            .unwrap(),
        target
    );
    assert_eq!(local.entries().len(), SINGLE_ITEM);
    assert_eq!(
        local.route_for(target, &observe()).unwrap().grant,
        restricted
    );
    assert!(local.route_for(target, &rights()).is_none());
    assert!(
        local
            .route_for(address(HUB_A, AGENT_TWO), &observe())
            .is_none()
    );
    let denied = Grant {
        rights: Rights::default(),
        remaining_reexports: REEXPORTS,
    };
    assert_eq!(
        local.register_local(agent(AGENT_TWO), denied),
        Err(RoutingError::EmptyRights)
    );
}

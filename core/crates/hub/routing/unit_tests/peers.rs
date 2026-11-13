use crate::support::{constants::*, *};
use yolo_hub_routing::{Grant, Rights, RoutingError};

#[test]
fn authorization_rejects_self_and_empty_grants() {
    let mut local = catalog(HUB_A);
    assert_eq!(
        local.authorize(hub(HUB_A), hub(HUB_B), grant()),
        Err(RoutingError::LocalPeer)
    );
    assert_eq!(
        local.authorize(hub(HUB_B), hub(HUB_A), grant()),
        Err(RoutingError::LocalOrigin)
    );
    let denied = Grant {
        rights: Rights::default(),
        remaining_reexports: REEXPORTS,
    };
    assert_eq!(
        local.authorize(hub(HUB_B), hub(HUB_B), denied),
        Err(RoutingError::EmptyRights)
    );
    assert_eq!(local.connect(hub(HUB_B)), Err(RoutingError::UnknownPeer));
}

#[test]
fn disconnect_and_reconnect_fence_prior_streams() {
    let mut local = catalog(HUB_A);
    let first = connected(&mut local, HUB_B, &[HUB_B]);
    assert_eq!(first.peer(), hub(HUB_B));
    let resource = advertisement(HUB_B, AGENT_ONE, &[HUB_B]);
    local
        .replace_from(first, FIRST_REVISION, vec![resource.clone()])
        .unwrap();
    local.disconnect(first).unwrap();
    assert_eq!(local.disconnect(first), Err(RoutingError::StaleConnection));
    assert_eq!(
        local.replace_from(first, SECOND_REVISION, vec![resource.clone()]),
        Err(RoutingError::StaleConnection)
    );
    assert!(!local.entries().first().unwrap().reachable);
    let second = local.connect(hub(HUB_B)).unwrap();
    assert_ne!(first, second);
    assert_eq!(local.disconnect(first), Err(RoutingError::StaleConnection));
    local
        .replace_from(second, FIRST_REVISION, vec![resource])
        .unwrap();
    assert!(local.entries().first().unwrap().reachable);
    let third = local.connect(hub(HUB_B)).unwrap();
    assert_ne!(second, third);
    assert!(!local.entries().first().unwrap().reachable);
}

#[test]
fn connection_tokens_cannot_cross_catalog_instances() {
    let mut first = catalog(HUB_A);
    let mut second = catalog(HUB_A);
    let old = connected(&mut first, HUB_B, &[HUB_B]);
    let current = connected(&mut second, HUB_B, &[HUB_B]);
    assert_ne!(old, current);
    assert_eq!(
        second.replace_from(old, FIRST_REVISION, Vec::new()),
        Err(RoutingError::StaleConnection)
    );
    second
        .replace_from(current, FIRST_REVISION, Vec::new())
        .unwrap();
}

#[test]
fn revocation_removes_only_the_affected_origin_and_eventually_the_peer() {
    let mut local = catalog(HUB_A);
    assert!(!local.revoke(hub(HUB_B), hub(HUB_C)));
    let connection = connected(&mut local, HUB_B, &[HUB_B, HUB_C]);
    local
        .replace_from(
            connection,
            FIRST_REVISION,
            vec![
                advertisement(HUB_B, AGENT_ONE, &[HUB_B]),
                advertisement(HUB_C, AGENT_TWO, &[HUB_B, HUB_C]),
            ],
        )
        .unwrap();
    assert!(!local.revoke(hub(HUB_B), hub(HUB_D)));
    assert!(local.revoke(hub(HUB_B), hub(HUB_C)));
    assert!(
        local
            .route_for(address(HUB_B, AGENT_ONE), &observe())
            .is_some()
    );
    assert!(
        local
            .route_for(address(HUB_C, AGENT_TWO), &observe())
            .is_none()
    );
    assert!(local.revoke(hub(HUB_B), hub(HUB_B)));
    assert_eq!(local.disconnect(connection), Err(RoutingError::UnknownPeer));
    assert_eq!(
        local.replace_from(connection, SECOND_REVISION, Vec::new()),
        Err(RoutingError::UnknownPeer)
    );
}

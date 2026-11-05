use crate::support::{constants::*, *};
use yolo_hub_routing::{Catalog, RoutingError};

#[test]
fn malformed_paths_cannot_change_the_catalog_or_consume_revision() {
    let cases = [
        (Vec::new(), RoutingError::EmptyPath),
        (vec![HUB_C], RoutingError::ForgedPeer),
        (vec![HUB_B], RoutingError::ForgedOrigin),
        (vec![HUB_B, HUB_A, HUB_C], RoutingError::CyclicPath),
        (vec![HUB_B, HUB_B, HUB_C], RoutingError::CyclicPath),
    ];
    for (path, expected) in cases {
        let mut local = catalog(HUB_A);
        let connection = connected(&mut local, HUB_B, &[HUB_C]);
        assert_eq!(
            local.replace_from(
                connection,
                FIRST_REVISION,
                vec![advertisement(HUB_C, AGENT_ONE, &path)]
            ),
            Err(expected)
        );
        assert!(local.entries().is_empty());
        local
            .replace_from(
                connection,
                FIRST_REVISION,
                vec![advertisement(HUB_C, AGENT_ONE, &[HUB_B, HUB_C])],
            )
            .unwrap();
    }
}

#[test]
fn paths_are_bounded_including_the_receiving_hub() {
    let mut bounded = limits();
    bounded.max_path_hubs = limit(TWO_ITEMS);
    let mut local = Catalog::new(hub(HUB_A), bounded);
    let connection = connected(&mut local, HUB_B, &[HUB_B, HUB_C]);
    assert_eq!(
        local.replace_from(
            connection,
            FIRST_REVISION,
            vec![advertisement(HUB_C, AGENT_ONE, &[HUB_B, HUB_C])]
        ),
        Err(RoutingError::PathLimit)
    );
    local
        .replace_from(
            connection,
            FIRST_REVISION,
            vec![advertisement(HUB_B, AGENT_ONE, &[HUB_B])],
        )
        .unwrap();
}

#[test]
fn peer_authentication_does_not_implicitly_authorize_transit_origins() {
    let mut local = catalog(HUB_A);
    let connection = connected(&mut local, HUB_B, &[HUB_B]);
    assert_eq!(
        local.replace_from(
            connection,
            FIRST_REVISION,
            vec![advertisement(HUB_C, AGENT_ONE, &[HUB_B, HUB_C])]
        ),
        Err(RoutingError::UnauthorizedOrigin)
    );
    assert!(local.entries().is_empty());
}

#[test]
fn duplicate_route_is_rejected_but_same_identity_on_distinct_paths_is_valid() {
    let mut local = catalog(HUB_A);
    let connection = connected(&mut local, HUB_B, &[HUB_C]);
    let direct = advertisement(HUB_C, AGENT_ONE, &[HUB_B, HUB_C]);
    assert_eq!(
        local.replace_from(
            connection,
            FIRST_REVISION,
            vec![direct.clone(), direct.clone()]
        ),
        Err(RoutingError::DuplicateRoute)
    );
    let indirect = advertisement(HUB_C, AGENT_ONE, &[HUB_B, HUB_D, HUB_C]);
    local
        .replace_from(connection, FIRST_REVISION, vec![indirect, direct.clone()])
        .unwrap();
    assert_eq!(local.entries().len(), SINGLE_ITEM);
    assert_eq!(
        local.route_for(direct.address, &observe()).unwrap().path,
        vec![hub(HUB_A), hub(HUB_B), hub(HUB_C)]
    );
}

use crate::support::{constants::*, *};
use yolo_hub_routing::{Right, Rights};

#[test]
fn multiple_paths_preserve_one_identity_and_choose_shortest_then_lexical() {
    let mut local = catalog(HUB_A);
    let via_b = connected(&mut local, HUB_B, &[HUB_C]);
    let via_d = connected(&mut local, HUB_D, &[HUB_C]);
    let resource = address(HUB_C, AGENT_ONE);
    local
        .replace_from(
            via_d,
            FIRST_REVISION,
            vec![advertisement(HUB_C, AGENT_ONE, &[HUB_D, HUB_C])],
        )
        .unwrap();
    local
        .replace_from(
            via_b,
            FIRST_REVISION,
            vec![advertisement(HUB_C, AGENT_ONE, &[HUB_B, HUB_C])],
        )
        .unwrap();
    assert_eq!(local.entries().len(), SINGLE_ITEM);
    assert_eq!(
        local.route_for(resource, &observe()).unwrap().next_hop(),
        Some(hub(HUB_B))
    );
    let direct = connected(&mut local, HUB_C, &[HUB_C]);
    local
        .replace_from(
            direct,
            FIRST_REVISION,
            vec![advertisement(HUB_C, AGENT_ONE, &[HUB_C])],
        )
        .unwrap();
    assert_eq!(
        local.route_for(resource, &observe()).unwrap().next_hop(),
        Some(hub(HUB_C))
    );
    local.disconnect(direct).unwrap();
    local.disconnect(via_b).unwrap();
    assert_eq!(
        local.route_for(resource, &observe()).unwrap().next_hop(),
        Some(hub(HUB_D))
    );
    local.disconnect(via_d).unwrap();
    assert_eq!(local.entries().first().unwrap().address, resource);
    assert!(!local.entries().first().unwrap().reachable);
}

#[test]
fn required_rights_choose_a_capable_route_without_combining_partial_grants() {
    let mut local = catalog(HUB_A);
    let direct = connected(&mut local, HUB_C, &[HUB_C]);
    let indirect = connected(&mut local, HUB_B, &[HUB_C]);
    let mut read_only = advertisement(HUB_C, AGENT_ONE, &[HUB_C]);
    read_only.grant.rights = observe();
    let mut write_only = advertisement(HUB_C, AGENT_ONE, &[HUB_B, HUB_C]);
    let write: Rights = [Right::TerminalWrite].into_iter().collect();
    write_only.grant.rights = write.clone();
    local
        .replace_from(direct, FIRST_REVISION, vec![read_only])
        .unwrap();
    local
        .replace_from(indirect, FIRST_REVISION, vec![write_only])
        .unwrap();
    let resource = address(HUB_C, AGENT_ONE);
    assert_eq!(
        local.route_for(resource, &observe()).unwrap().next_hop(),
        Some(hub(HUB_C))
    );
    assert_eq!(
        local.route_for(resource, &write).unwrap().next_hop(),
        Some(hub(HUB_B))
    );
    let combined = [Right::Observe, Right::TerminalWrite].into_iter().collect();
    assert!(local.route_for(resource, &combined).is_none());
}

#[test]
fn wire_advertisement_roundtrip_preserves_original_address_and_path() {
    let advertisement = advertisement(HUB_C, AGENT_ONE, &[HUB_B, HUB_C]);
    let encoded = serde_json::to_string(&advertisement).unwrap();
    assert_eq!(
        serde_json::from_str::<yolo_hub_routing::RouteAdvertisement>(&encoded).unwrap(),
        advertisement
    );
}

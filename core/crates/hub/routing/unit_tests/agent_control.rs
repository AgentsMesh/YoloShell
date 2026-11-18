use crate::support::{constants::*, *};
use yolo_hub_routing::{Grant, HubCatalog, Right, Rights, RouteAdvertisement};

#[test]
fn agent_control_and_terminal_write_permissions_cannot_be_combined_across_routes() {
    let mut local = HubCatalog::new(hub(HUB_A), limits());
    let control: Rights = [Right::AgentControl].into_iter().collect();
    let write: Rights = [Right::TerminalWrite].into_iter().collect();
    let required = control.union(&write);
    let mut links = Vec::new();
    for (peer, rights) in [(HUB_B, control.clone()), (HUB_D, write.clone())] {
        local.authorize(hub(peer), hub(HUB_C), grant()).unwrap();
        let connection = local.connect(hub(peer)).unwrap();
        local
            .replace_from(
                connection,
                FIRST_REVISION,
                vec![RouteAdvertisement {
                    address: hub(HUB_C),
                    path: vec![hub(peer), hub(HUB_C)],
                    grant: Grant {
                        rights,
                        remaining_reexports: SINGLE_REEXPORT,
                    },
                }],
            )
            .unwrap();
        links.push(connection);
    }
    assert!(local.route_for(hub(HUB_C), &control).is_some());
    assert!(local.route_for(hub(HUB_C), &write).is_some());
    assert!(local.route_for(hub(HUB_C), &required).is_none());
    let complete = RouteAdvertisement {
        address: hub(HUB_C),
        path: vec![hub(HUB_D), hub(HUB_C)],
        grant: Grant {
            rights: required.clone(),
            remaining_reexports: SINGLE_REEXPORT,
        },
    };
    local
        .replace_from(links[1], SECOND_REVISION, vec![complete])
        .unwrap();
    assert_eq!(
        local.route_for(hub(HUB_C), &required).unwrap().next_hop(),
        Some(hub(HUB_D))
    );
    local.disconnect(links[1]).unwrap();
    assert!(local.route_for(hub(HUB_C), &required).is_none());
    assert!(local.route_for(hub(HUB_C), &control).is_some());
}

#[test]
fn previously_issued_terminal_write_grant_does_not_acquire_semantic_right_on_decode() {
    let previous: Rights = [
        Right::Observe,
        Right::TerminalRead,
        Right::TerminalWrite,
        Right::Lifecycle,
        Right::FileRead,
    ]
    .into_iter()
    .collect();
    let encoded = serde_json::to_vec(&previous).unwrap();
    let restored: Rights = serde_json::from_slice(&encoded).unwrap();
    assert!(!restored.allows(&[Right::AgentControl].into_iter().collect()));
    let independent: Rights = [Right::AgentControl].into_iter().collect();
    let restored: Rights =
        serde_json::from_slice(&serde_json::to_vec(&independent).unwrap()).unwrap();
    assert_eq!(restored, independent);
    assert!(!restored.allows(&[Right::TerminalWrite].into_iter().collect()));
}

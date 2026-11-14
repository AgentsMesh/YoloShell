use super::{list_hubs, page};
#[path = "../test_support/inventory.rs"]
mod support;
use std::collections::BTreeMap;
use support::*;
use yolo_hub_federation_directory::OriginBook;
use yolo_hub_federation_settings::routing_limits;
use yolo_hub_peer_protocol::{FIRST_REVISION, Purpose, Signed};
use yolo_hub_peer_protocol_test_support::{
    Fixture,
    support::{NOW, STEP},
};
use yolo_hub_rpc_protocol::{Command, MAX_CATALOG_PAGE_HUBS, Reply, Request, Response};

#[test]
fn aggregate_is_sorted_deduplicated_and_local_reachability_is_not_overridden() {
    let reply = page(
        hub(LOCAL_ID),
        vec![
            summary(LAST_ID, false),
            summary(REMOTE_ID, false),
            summary(LOCAL_ID, false),
            summary(REMOTE_ID, true),
        ],
        None,
        &BTreeMap::new(),
    );
    assert_eq!(
        reply,
        Reply::Hubs {
            hubs: vec![
                summary(LOCAL_ID, true),
                summary(REMOTE_ID, true),
                summary(LAST_ID, false)
            ],
            next: None
        }
    );
    let request = Request::new(hub(LOCAL_ID), Command::ListHubs { after: None }).unwrap();
    assert!(
        Response::new(&request, hub(LOCAL_ID), reply)
            .validate_for(&request)
            .is_ok()
    );
}

#[test]
fn aggregate_pagination_is_strictly_after_and_does_not_duplicate_any_origin() {
    for count in [MAX_CATALOG_PAGE_HUBS, MAX_CATALOG_PAGE_HUBS + ITEM_STEP] {
        let remote: Vec<_> = (ITEM_STEP..=count)
            .map(|value| summary(value as u128, false))
            .collect();
        let first = page(hub(LOCAL_ID), remote.clone(), None, &BTreeMap::new());
        let Reply::Hubs { hubs, next } = first else {
            panic!()
        };
        assert_eq!(hubs.len(), MAX_CATALOG_PAGE_HUBS);
        assert_eq!(next.is_some(), count > MAX_CATALOG_PAGE_HUBS);
        if let Some(cursor) = next {
            let second = page(hub(LOCAL_ID), remote.clone(), Some(cursor), &BTreeMap::new());
            assert_eq!(
                second,
                Reply::Hubs {
                    hubs: vec![summary(count as u128, false)],
                    next: None
                }
            );
            let request = Request::new(
                hub(LOCAL_ID),
                Command::ListHubs {
                    after: Some(cursor),
                },
            )
            .unwrap();
            assert!(
                Response::new(&request, hub(LOCAL_ID), second)
                    .validate_for(&request)
                    .is_ok()
            );
        }
        assert_eq!(
            page(hub(LOCAL_ID), remote, Some(hub(count as u128)), &BTreeMap::new()),
            Reply::Hubs {
                hubs: vec![],
                next: None
            }
        );
    }
}

#[test]
fn aggregate_includes_authenticated_empty_origin_but_not_its_undiscovered_relay() {
    let fixture = Fixture::new();
    let local = fixture.entry.identity.hub();
    let mut book = OriginBook::new(
        fixture.entry.identity,
        vec![fixture.relay.identity, fixture.origin.identity],
        routing_limits(),
    )
    .unwrap();
    assert_eq!(
        list_hubs(local, &book, NOW, None, &BTreeMap::new()).unwrap(),
        Reply::Hubs {
            hubs: vec![yolo_hub_rpc_protocol::HubSummary {
                hub_id: local,
                reachable: true,
                label: None,
            }],
            next: None
        }
    );
    let mut export = fixture.export.clone();
    let mut empty = export.catalogue.payload.clone();
    empty.terminals.clear();
    export.catalogue = Signed::new(empty, Purpose::Catalogue, &fixture.origin).unwrap();
    let link = uuid::Uuid::new_v4();
    book.connect(fixture.relay.identity, link).unwrap();
    book.install(
        fixture.relay.identity,
        link,
        FIRST_REVISION,
        vec![export],
        NOW,
    )
    .unwrap();
    let Reply::Hubs { hubs, .. } = list_hubs(local, &book, NOW, None, &BTreeMap::new()).unwrap() else {
        panic!()
    };
    assert!(
        hubs.iter()
            .any(|hub| hub.hub_id == fixture.origin.identity.hub() && hub.reachable)
    );
    assert!(
        !hubs
            .iter()
            .any(|hub| hub.hub_id == fixture.relay.identity.hub())
    );
    book.install(
        fixture.relay.identity,
        link,
        FIRST_REVISION + STEP,
        vec![],
        NOW,
    )
    .unwrap();
    let Reply::Hubs { hubs, .. } = list_hubs(local, &book, NOW, None, &BTreeMap::new()).unwrap() else {
        panic!()
    };
    assert!(
        hubs.iter()
            .any(|hub| hub.hub_id == fixture.origin.identity.hub() && !hub.reachable)
    );
    assert!(hubs.iter().any(|hub| hub.hub_id == local && hub.reachable));
}

#[test]
fn configured_peer_labels_attach_without_labeling_the_local_hub() {
    let labels = BTreeMap::from([(hub(REMOTE_ID), String::from("build-box"))]);
    let reply = page(
        hub(LOCAL_ID),
        vec![summary(REMOTE_ID, true)],
        None,
        &labels,
    );
    let Reply::Hubs { hubs, .. } = reply else {
        panic!()
    };
    assert_eq!(
        hubs.iter()
            .find(|row| row.hub_id == hub(REMOTE_ID))
            .and_then(|hub| hub.label.as_deref()),
        Some("build-box")
    );
    assert!(
        hubs.iter()
            .find(|row| row.hub_id == hub(LOCAL_ID))
            .is_some_and(|row| row.label.is_none())
    );
}

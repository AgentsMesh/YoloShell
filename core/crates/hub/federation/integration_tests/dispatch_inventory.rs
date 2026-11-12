#[path = "../test_support/node_inventory.rs"]
mod support;
use support::NodeFixture;
use yolo_hub_dispatch::{Dispatcher, Operation};
use yolo_hub_rpc_protocol::{Command, HubSummary, Reply, Request, Response};

#[tokio::test]
async fn public_entry_aggregates_empty_hubs_while_origin_backend_does_not_export_topology() {
    let fixture = NodeFixture::new().await;
    let request = Request::new(fixture.node.hub_id(), Command::ListHubs { after: None }).unwrap();
    let Operation::Reply(reply) = fixture.node.dispatch(request.clone()).await.unwrap() else {
        panic!()
    };
    assert!(
        Response::new(&request, fixture.node.hub_id(), reply.clone())
            .validate_for(&request)
            .is_ok()
    );
    let Reply::Hubs { hubs, next } = reply else {
        panic!()
    };
    assert_eq!(next, None);
    let mut expected = vec![
        HubSummary {
            hub_id: fixture.node.hub_id(),
            reachable: true,
            label: None,
        },
        HubSummary {
            hub_id: fixture.origin.identity.hub(),
            reachable: true,
            label: None,
        },
    ];
    expected.sort_by_key(|hub| hub.hub_id);
    assert_eq!(hubs, expected);
    // The authenticated peer-origin path dispatches to backend, not Federation.
    let Operation::Reply(local) = fixture.node.backend.dispatch(request).await.unwrap() else {
        panic!()
    };
    assert_eq!(
        local,
        Reply::Hubs {
            hubs: vec![HubSummary {
                hub_id: fixture.node.hub_id(),
                reachable: true,
                label: None,
            }],
            next: None
        }
    );
    assert!(!fixture.host_socket.exists());
    fixture
        .node
        .book
        .lock()
        .await
        .disconnect(fixture.relay.identity.hub(), fixture.link)
        .unwrap();
    let request = Request::new(fixture.node.hub_id(), Command::ListHubs { after: None }).unwrap();
    let Operation::Reply(Reply::Hubs { hubs, .. }) = fixture.node.dispatch(request).await.unwrap()
    else {
        panic!()
    };
    assert!(
        hubs.iter()
            .any(|hub| hub.hub_id == fixture.origin.identity.hub() && !hub.reachable)
    );
}

#[path = "../test_support/node_inventory.rs"]
mod fixture;
use fixture::NodeFixture;
use std::sync::Arc;
use yolo_hub_federation_directory::OriginBook;
use yolo_hub_federation_settings::routing_limits;
use yolo_hub_peer_protocol::{Call, MAX_CALL_LIFETIME_MS, PEER_PROTOCOL_VERSION, Purpose, Signed};
use yolo_hub_peer_protocol_test_support::support::observe;
use yolo_hub_rpc_protocol::{Command, Request};
use yolo_hub_terminal_protocol::PublicError;

#[tokio::test]
async fn shutdown_fence_is_monotonic_and_rejects_admission_publication_and_forwarding() {
    let fixture = NodeFixture::new().await;
    let node = &fixture.node;
    let now = crate::node::now_ms().unwrap();
    let origin = fixture.origin.identity.hub();
    let route = node
        .book
        .lock()
        .await
        .route_for(origin, &observe(), now)
        .unwrap()
        .unwrap();
    let local = Signed::new(
        route.catalogue.payload.clone(),
        Purpose::Catalogue,
        node.state.as_ref(),
    )
    .unwrap();
    node.latest.send_replace(Some(Arc::new(local)));
    assert!(node.source().is_some());
    let call = Signed::new(
        Call {
            request: Request::new(origin, Command::ListHubs { after: None }).unwrap(),
            authority: route.authority.clone(),
            issued_at_ms: now,
            expires_at_ms: now + MAX_CALL_LIFETIME_MS,
        },
        Purpose::Call,
        node.state.as_ref(),
    )
    .unwrap();
    node.begin_shutdown();
    node.begin_shutdown();
    assert!(node.is_closing());
    assert!(node.require_open().is_err());
    assert!(node.source().is_none());
    assert_eq!(
        node.admit(fixture.relay.identity, &call).await,
        Err(PublicError::Unreachable)
    );
    assert!(matches!(
        node.forward(&call).await,
        Err(PublicError::Unreachable)
    ));
    assert!(
        node.directory(
            fixture.relay.identity,
            uuid::Uuid::new_v4(),
            PEER_PROTOCOL_VERSION,
            vec![origin]
        )
        .await
        .is_err()
    );
    assert!(node.restore_history(node.history().await).await.is_err());
    assert!(!fixture.host_socket.exists());
}

#[tokio::test]
async fn federation_history_roundtrip_restores_only_offline_catalogues() {
    let fixture = NodeFixture::new().await;
    let node = &fixture.node;
    let history = node.history().await;
    let now = crate::node::now_ms().unwrap();
    assert!(
        tokio::time::timeout(crate::OPERATION_TIMEOUT, node.copy_history_from(node))
            .await
            .unwrap()
            .is_err()
    );
    *node.book.lock().await = OriginBook::new(
        node.state.peer_identity(),
        node.config.pins(),
        routing_limits(),
    )
    .unwrap();
    node.restore_history(history).await.unwrap();
    let book = node.book.lock().await;
    assert!(book.latest_for(fixture.origin.identity.hub()).is_some());
    assert!(
        book.route_for(fixture.origin.identity.hub(), &observe(), now)
            .unwrap()
            .is_none()
    );
    assert!(book.exportable(now).unwrap().is_empty());
    assert!(book.hubs(now).unwrap().iter().all(|hub| !hub.reachable));
    drop(book);
    assert!(node.connections.read().await.is_empty());
    assert!(!fixture.host_socket.exists());
}

#[tokio::test]
async fn copying_an_empty_history_to_itself_does_not_hold_two_directory_locks() {
    let fixture = NodeFixture::new().await;
    let node = &fixture.node;
    *node.book.lock().await = OriginBook::new(
        node.state.peer_identity(),
        node.config.pins(),
        routing_limits(),
    )
    .unwrap();
    tokio::time::timeout(crate::OPERATION_TIMEOUT, node.copy_history_from(node))
        .await
        .unwrap()
        .unwrap();
    assert!(
        node.book
            .lock()
            .await
            .hubs(crate::node::now_ms().unwrap())
            .unwrap()
            .is_empty()
    );
}

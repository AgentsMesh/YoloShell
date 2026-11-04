use std::{
    future::poll_fn,
    sync::{Arc, Mutex},
    task::Poll,
};
use tokio::time::{Instant, timeout};
use yolo_hub_peer_protocol::{PEER_PROTOCOL_VERSION, PeerRequest};
use yolo_hub_peer_rpc::{Client, Handler};

#[path = "../test_support/node_inventory.rs"]
mod fixture;
#[path = "../test_support/lifetime_gate.rs"]
mod gate;
#[path = "../test_support/lifetime_source.rs"]
mod source;
#[path = "../test_support/lifetime_bridge.rs"]
mod support;
use gate::*;
use support::*;

async fn open(handler: Arc<dyn Handler>) -> (Client, tokio::task::JoinHandle<()>) {
    let (client, server) = tokio::io::duplex(PIPE_BYTES);
    let task = tokio::spawn(async move {
        let _ = yolo_hub_peer_rpc::serve(server, handler).await;
    });
    (
        Client::connect(client, Instant::now() + TEST_TIMEOUT)
            .await
            .unwrap(),
        task,
    )
}
fn request() -> PeerRequest {
    PeerRequest::Discover {
        request_id: uuid::Uuid::new_v4(),
        version: PEER_PROTOCOL_VERSION,
        allowed_origins: Vec::new(),
    }
}

#[tokio::test]
async fn generation_drain_waits_for_bridge_capability_teardown_on_the_rpc_executor() {
    let fixture = fixture::NodeFixture::new().await;
    let (call, _) = source::call(&fixture.node, &fixture.relay);
    let (upstream, serving) = open(Arc::new(UpdateHandler(Mutex::new(Some(Box::new(
        PendingUpdates,
    ))))))
    .await;
    let updates = upstream
        .request(request(), Instant::now() + TEST_TIMEOUT)
        .await
        .unwrap()
        .updates
        .unwrap();
    drop(upstream);
    let bridge = fixture.node.clone().bridge(call, updates).unwrap();
    let (mut gate, delayed) = Gate::new();
    let (downstream, exporting) = open(Arc::new(UpdateHandler(Mutex::new(Some(Box::new(
        DelayedBridge {
            _gate: delayed,
            updates: bridge,
        },
    ))))))
    .await;
    let capability = downstream
        .request(request(), Instant::now() + TEST_TIMEOUT)
        .await
        .unwrap();
    assert!(capability.updates.is_some());
    fixture.node.begin_shutdown();
    exporting.abort();
    let _ = exporting.await;
    timeout(TEST_TIMEOUT, &mut gate.entered)
        .await
        .unwrap()
        .unwrap();
    let draining = fixture.node.wait_retired(Instant::now() + TEST_TIMEOUT);
    tokio::pin!(draining);
    poll_fn(|cx| {
        assert!(draining.as_mut().poll(cx).is_pending());
        Poll::Ready(())
    })
    .await;
    gate.release();
    draining.await.unwrap();
    assert_eq!(Arc::strong_count(&fixture.node), 1);
    drop(capability);
    drop(downstream);
    timeout(TEST_TIMEOUT, serving).await.unwrap().unwrap();
    fixture
        .node
        .observation_cache
        .wait_closed(Instant::now() + TEST_TIMEOUT)
        .await;
}

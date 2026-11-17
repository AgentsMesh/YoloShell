use super::PeerHandler;
use crate::lifetime::OwnedGeneration;
use std::{future::poll_fn, sync::Arc, task::Poll};
use tokio::{
    sync::Notify,
    time::{Instant, timeout},
};
use yolo_hub_peer_protocol::{PEER_PROTOCOL_VERSION, PeerRequest};
use yolo_hub_peer_rpc::{Client, Handler, Response, RpcFuture};

#[path = "../test_support/node_inventory.rs"]
mod fixture;
#[path = "../test_support/lifetime_gate.rs"]
mod gate;
use gate::*;

struct DelayedHandler {
    _gate: DropGate,
    inner: PeerHandler,
    entered: Arc<Notify>,
}
impl Handler for DelayedHandler {
    fn request(&self, request: PeerRequest) -> RpcFuture<'_, Response> {
        self.entered.notify_one();
        self.inner.request(request)
    }
}

#[tokio::test]
async fn retired_generation_waits_for_actual_pending_peer_rpc_domain_teardown() {
    let fixture = fixture::NodeFixture::new().await;
    let node = &fixture.node;
    let book = node.book.lock().await;
    let entered = Arc::new(Notify::new());
    let (mut gate, delayed) = Gate::new();
    let handler = Arc::new(DelayedHandler {
        _gate: delayed,
        inner: PeerHandler {
            peer: fixture.relay.identity,
            generation: OwnedGeneration::new(node.clone()).unwrap(),
        },
        entered: entered.clone(),
    });
    let (client, server) = tokio::io::duplex(PIPE_BYTES);
    let domain = DomainTask::spawn(async move {
        let _ = yolo_hub_peer_rpc::serve(server, handler).await;
    })
    .await;
    let client = Client::connect(client, Instant::now() + TEST_TIMEOUT)
        .await
        .unwrap();
    let request = tokio::spawn(async move {
        client
            .request(
                PeerRequest::Discover {
                    request_id: uuid::Uuid::new_v4(),
                    version: PEER_PROTOCOL_VERSION,
                    allowed_origins: Vec::new(),
                },
                Instant::now() + TEST_TIMEOUT,
            )
            .await
    });
    timeout(TEST_TIMEOUT, entered.notified()).await.unwrap();
    node.begin_shutdown();
    domain.abort.abort();
    timeout(TEST_TIMEOUT, &mut gate.entered)
        .await
        .unwrap()
        .unwrap();
    let draining = node.wait_retired(Instant::now() + TEST_TIMEOUT);
    tokio::pin!(draining);
    poll_fn(|cx| {
        assert!(draining.as_mut().poll(cx).is_pending());
        Poll::Ready(())
    })
    .await;
    gate.release();
    draining.await.unwrap();
    assert_eq!(Arc::strong_count(node), 1);
    drop(book);
    domain.finish().await;
    assert!(
        timeout(TEST_TIMEOUT, request)
            .await
            .unwrap()
            .unwrap()
            .is_err()
    );
    node.observation_cache
        .wait_closed(Instant::now() + TEST_TIMEOUT)
        .await;
}

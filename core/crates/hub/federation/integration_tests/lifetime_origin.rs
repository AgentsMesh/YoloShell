use std::{future::poll_fn, sync::Arc, task::Poll};
use tokio::{
    sync::{Notify, oneshot},
    time::{Instant, timeout},
};

#[path = "../test_support/node_inventory.rs"]
mod fixture;
#[path = "../test_support/lifetime_gate.rs"]
mod gate;
#[path = "../test_support/lifetime_source.rs"]
mod source;
use gate::*;

#[path = "origin_events.rs"]
mod events;

#[tokio::test]
async fn generation_drain_waits_for_cancelled_origin_producers_actual_source_drop() {
    let fixture = fixture::NodeFixture::new().await;
    let (call, initial) = source::call(&fixture.node, &fixture.relay);
    let entered = Arc::new(Notify::new());
    let (mut gate, delayed) = Gate::new();
    let source = source::DelayedSource {
        _gate: delayed,
        initial,
        entered: entered.clone(),
    };
    let (ready, updates) = oneshot::channel();
    let (finish, finished) = oneshot::channel::<()>();
    let node = fixture.node.clone();
    let domain = DomainTask::spawn(async move {
        let updates = node.origin_stream(call, Box::new(source)).unwrap();
        assert!(ready.send(updates).is_ok());
        let _ = finished.await;
    })
    .await;
    let updates = timeout(TEST_TIMEOUT, updates).await.unwrap().unwrap();
    timeout(TEST_TIMEOUT, entered.notified()).await.unwrap();
    fixture.node.begin_shutdown();
    drop(updates);
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
    finish.send(()).unwrap();
    domain.finish().await;
    fixture
        .node
        .observation_cache
        .wait_closed(Instant::now() + TEST_TIMEOUT)
        .await;
}

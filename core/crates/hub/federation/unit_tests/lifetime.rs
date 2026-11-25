use super::*;
use std::{future::poll_fn, task::Poll};

#[path = "../test_support/node_inventory.rs"]
mod fixture;

const TEST_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(3);

#[tokio::test]
async fn retirement_rejects_new_owners_and_waits_for_every_accepted_owner() {
    let lifetime = Arc::new(Lifetime::default());
    let first = lifetime.acquire().unwrap();
    let second = lifetime.acquire().unwrap();
    assert!(lifetime.drained(Instant::now()).await.is_err());
    lifetime.retire();
    lifetime.retire();
    assert!(lifetime.acquire().is_err());
    let drained = lifetime.drained(Instant::now() + TEST_TIMEOUT);
    tokio::pin!(drained);
    poll_fn(|cx| {
        assert!(drained.as_mut().poll(cx).is_pending());
        Poll::Ready(())
    })
    .await;
    drop(first);
    poll_fn(|cx| {
        assert!(drained.as_mut().poll(cx).is_pending());
        Poll::Ready(())
    })
    .await;
    drop(second);
    drained.await.unwrap();
}

#[tokio::test]
async fn timed_out_or_cancelled_waiter_does_not_release_or_reopen_custody() {
    let lifetime = Arc::new(Lifetime::default());
    let ticket = lifetime.acquire().unwrap();
    lifetime.retire();
    assert!(lifetime.drained(Instant::now()).await.is_err());
    let mut cancelled = Box::pin(lifetime.drained(Instant::now() + TEST_TIMEOUT));
    poll_fn(|cx| {
        assert!(cancelled.as_mut().poll(cx).is_pending());
        Poll::Ready(())
    })
    .await;
    drop(cancelled);
    assert!(lifetime.acquire().is_err());
    assert!(lifetime.drained(Instant::now()).await.is_err());
    drop(ticket);
    lifetime
        .drained(Instant::now() + TEST_TIMEOUT)
        .await
        .unwrap();
}

#[tokio::test]
async fn activity_completion_is_published_after_the_owned_generation_is_dropped() {
    let fixture = fixture::NodeFixture::new().await;
    let node = fixture.node.clone();
    let lifetime = node.lifetime.clone();
    let weak = Arc::downgrade(&node);
    let owner = OwnedGeneration::new(node.clone()).unwrap();
    node.begin_shutdown();
    node.observation_cache
        .wait_closed(Instant::now() + TEST_TIMEOUT)
        .await;
    assert!(OwnedGeneration::new(node.clone()).is_err());
    drop(fixture.node);
    drop(node);
    assert!(weak.upgrade().is_some());
    drop(owner);
    lifetime
        .drained(Instant::now() + TEST_TIMEOUT)
        .await
        .unwrap();
    assert!(weak.upgrade().is_none());
}

#[test]
fn counter_exhaustion_does_not_create_untracked_ownership() {
    let lifetime = Arc::new(Lifetime::default());
    lifetime.state.lock().unwrap().active = usize::MAX;
    assert!(lifetime.acquire().is_err());
    assert_eq!(lifetime.state.lock().unwrap().active, usize::MAX);
}

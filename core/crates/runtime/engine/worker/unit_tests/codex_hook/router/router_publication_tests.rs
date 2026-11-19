use super::support::{Fixture, ready_retirement};
use std::sync::{Arc, Barrier};

const SUBSCRIPTION_RACE_ROUNDS: usize = 16;
const REVISION_INCREMENT: u64 = 1;

#[tokio::test]
async fn provisional_route_is_published_only_after_successful_settlement() {
    let fixture = Fixture::new();
    let router = fixture.router();
    let authority = fixture.authority();
    let lease = authority.install(&router).unwrap();
    let mut subscription = router.subscribe_committed().unwrap();

    assert!(subscription.snapshot.is_empty());
    assert!(matches!(
        subscription.receiver.try_recv(),
        Err(tokio::sync::broadcast::error::TryRecvError::Empty)
    ));

    lease.settle().unwrap();
    let published = subscription.receiver.recv().await.unwrap();
    assert_eq!(
        published.entry.key().invocation_id(),
        authority.identity.invocation_id
    );
}

#[tokio::test]
async fn exact_replay_keeps_revision_and_does_not_publish_a_second_route() {
    let fixture = Fixture::new();
    let router = fixture.router();
    let first = fixture.authority();
    first.install(&router).unwrap().settle().unwrap();
    let mut subscription = router.subscribe_committed().unwrap();
    let first_revision = subscription.snapshot[0].revision;

    let replay = first.install(&router).unwrap();
    assert!(replay.is_replay());
    replay.settle().unwrap();
    assert!(matches!(
        subscription.receiver.try_recv(),
        Err(tokio::sync::broadcast::error::TryRecvError::Empty)
    ));

    let successor = fixture.authority();
    successor.install(&router).unwrap().settle().unwrap();
    let publication = subscription.receiver.recv().await.unwrap();
    assert_eq!(publication.revision, first_revision + REVISION_INCREMENT);
}

#[test]
fn subscribe_before_snapshot_never_loses_a_concurrent_settlement() {
    let fixture = Fixture::new();
    let router = Arc::new(fixture.router());
    for _ in 0..SUBSCRIPTION_RACE_ROUNDS {
        let authority = fixture.authority();
        let lease = authority.install(&router).unwrap();
        let expected = authority.identity.invocation_id;
        let barrier = Arc::new(Barrier::new(2));
        let subscription_router = Arc::clone(&router);
        let subscription_barrier = Arc::clone(&barrier);
        let subscriber = std::thread::spawn(move || {
            subscription_barrier.wait();
            subscription_router.subscribe_committed().unwrap()
        });
        barrier.wait();
        lease.settle().unwrap();
        let mut subscription = subscriber.join().unwrap();
        let in_snapshot = subscription
            .snapshot
            .iter()
            .any(|route| route.entry.key().invocation_id() == expected);
        let in_stream = std::iter::from_fn(|| subscription.receiver.try_recv().ok())
            .any(|route| route.entry.key().invocation_id() == expected);
        assert!(in_snapshot || in_stream);
    }
}

#[test]
fn subscriber_reports_lag_after_the_bounded_publication_window_is_overrun() {
    let fixture = Fixture::new();
    let publication_count = super::super::MAX_ACTIVE_CODEX_INVOCATIONS + 1;
    let router = fixture.router_with_capacity(publication_count);
    let mut subscription = router.subscribe_committed().unwrap();
    for _ in 0..publication_count {
        let authority = fixture.authority();
        let lease = authority.install(&router).unwrap();
        let entry = Arc::clone(lease.entry());
        lease.settle().unwrap();
        ready_retirement(router.begin_retirement(&entry))
            .commit()
            .unwrap();
    }

    assert!(matches!(
        subscription.receiver.try_recv(),
        Err(tokio::sync::broadcast::error::TryRecvError::Lagged(skipped)) if skipped > 0
    ));
}

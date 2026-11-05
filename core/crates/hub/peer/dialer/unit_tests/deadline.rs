use super::{constants::*, fixture::*, support::*};
use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};
use tokio::time::{Instant, timeout_at};
use yolo_hub_peer_dialer::dial_peer;
use yolo_hub_peer_dialer::PeerIo;

#[tokio::test]
async fn deadline_and_full_configured_identity_are_preserved_without_authentication_in_factory() {
    let (stream, _remote) = tokio::io::duplex(BUFFER_BYTES);
    let closed = Arc::new(AtomicUsize::default());
    let observed = closed.clone();
    let factory = Injected::new(
        Some(PeerIo::new(stream, move || {
            observed.fetch_add(ONE, Ordering::SeqCst);
        })),
        Behavior::Ready,
    );
    let peer = identity();
    let address = address();
    let deadline = deadline();
    let transport = dial_peer(&factory, peer, &address, deadline).await.unwrap();
    assert_eq!(
        *factory.seen.lock().unwrap(),
        vec![(peer, address, deadline)]
    );
    assert_eq!(closed.load(Ordering::SeqCst), 0);
    drop(transport);
    assert_eq!(closed.load(Ordering::SeqCst), ONE);
}

#[tokio::test]
async fn invalid_input_or_expired_budget_never_polls_factory() {
    let factory = Injected::new(None, Behavior::Reject);
    let peer = identity();
    assert!(
        dial_peer(&factory, peer, &address(), Instant::now())
            .await
            .is_err()
    );
    let mut invalid = address();
    invalid.socket = "relative.sock".into();
    assert!(
        dial_peer(&factory, peer, &invalid, deadline())
            .await
            .is_err()
    );
    assert_eq!(factory.calls.load(Ordering::SeqCst), 0);
}

#[tokio::test]
async fn pending_factory_is_cancelled_at_the_shared_absolute_deadline_and_releases_io() {
    let (stream, _remote) = tokio::io::duplex(BUFFER_BYTES);
    let closed = Arc::new(AtomicUsize::default());
    let observed = closed.clone();
    let factory = Injected::new(
        Some(PeerIo::new(stream, move || {
            observed.fetch_add(ONE, Ordering::SeqCst);
        })),
        Behavior::Pending,
    );
    assert!(
        dial_peer(
            &factory,
            identity(),
            &address(),
            Instant::now() + SHORT_TIMEOUT
        )
        .await
        .is_err()
    );
    assert_eq!(factory.calls.load(Ordering::SeqCst), ONE);
    assert_eq!(closed.load(Ordering::SeqCst), ONE);
}

#[tokio::test]
async fn caller_cancellation_drops_the_inflight_factory_transport_without_detached_work() {
    let (stream, _remote) = tokio::io::duplex(BUFFER_BYTES);
    let closed = Arc::new(AtomicUsize::default());
    let observed = closed.clone();
    let factory = Arc::new(Injected::new(
        Some(PeerIo::new(stream, move || {
            observed.fetch_add(ONE, Ordering::SeqCst);
        })),
        Behavior::Pending,
    ));
    let opening_factory = factory.clone();
    let peer = identity();
    let opening = tokio::spawn(async move {
        dial_peer(opening_factory.as_ref(), peer, &address(), deadline()).await
    });
    timeout_at(deadline(), factory.entered.notified())
        .await
        .unwrap();
    opening.abort();
    assert!(matches!(opening.await, Err(error) if error.is_cancelled()));
    assert_eq!(closed.load(Ordering::SeqCst), ONE);
}

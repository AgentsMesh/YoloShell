use super::*;
use std::{sync::mpsc, time::Duration};
use yolo_hub_federation_settings::routing_limits;
use yolo_hub_peer_protocol_test_support::support::Identity;

const DEADLINE: Duration = Duration::from_millis(20);
const BLOCKED_IO: &str = "deliberately blocked observation storage";

#[tokio::test]
async fn stalled_startup_retires_without_restoring_or_waiting_for_the_disk_thread() {
    let mut book = OriginBook::new(Identity::new().identity, Vec::new(), routing_limits()).unwrap();
    let original = book.history();
    let (release, blocked) = mpsc::channel();
    let (entered, started) = oneshot::channel();
    let cache = ObservationCache::start(
        book.history().entry().hub(),
        move |retirement| {
            let _ = entered.send(retirement.clone());
            let _ = blocked.recv();
            assert!(retirement.is_retired());
            anyhow::bail!(BLOCKED_IO)
        },
        &mut book,
        Instant::now() + DEADLINE,
    )
    .await;
    let retirement = timeout_at(Instant::now() + STOP_TIMEOUT, started)
        .await
        .unwrap()
        .unwrap();
    assert!(retirement.is_retired());
    assert!(cache.record(original.clone()).is_none());
    assert_eq!(book.history(), original);
    cache.wait_closed(Instant::now() + DEADLINE).await;
    assert!(!*cache.closed.borrow());
    release.send(()).unwrap();
    cache.wait_closed(Instant::now() + STOP_TIMEOUT).await;
    assert!(*cache.closed.borrow());
}

#[tokio::test]
async fn cancelling_startup_retires_the_worker_and_does_not_wait_for_blocked_io() {
    let (release, blocked) = mpsc::channel();
    let (entered, started) = oneshot::channel();
    let task = tokio::spawn(async move {
        let mut book =
            OriginBook::new(Identity::new().identity, Vec::new(), routing_limits()).unwrap();
        ObservationCache::start(
            book.history().entry().hub(),
            move |retirement| {
                let _ = entered.send(retirement.clone());
                let _ = blocked.recv();
                assert!(retirement.is_retired());
                anyhow::bail!(BLOCKED_IO)
            },
            &mut book,
            Instant::now() + STOP_TIMEOUT,
        )
        .await
    });
    let retirement = timeout_at(Instant::now() + STOP_TIMEOUT, started)
        .await
        .unwrap()
        .unwrap();
    task.abort();
    assert!(matches!(task.await, Err(error) if error.is_cancelled()));
    assert!(retirement.is_retired());
    release.send(()).unwrap();
}

#[tokio::test]
async fn stalled_write_keeps_book_available_and_retirement_does_not_replay_pending_data() {
    let book = tokio::sync::Mutex::new(
        OriginBook::new(Identity::new().identity, Vec::new(), routing_limits()).unwrap(),
    );
    let history = book.lock().await.history();
    let mailbox = Arc::new(Mailbox::default());
    assert!(mailbox.activate());
    let (finished, closed) = watch::channel(false);
    let cache = ObservationCache {
        mailbox: mailbox.clone(),
        closed,
    };
    let (entered, started) = oneshot::channel();
    let (release, blocked) = mpsc::channel();
    std::thread::spawn(move || {
        let mut entered = Some(entered);
        let result = observation_worker::persist(&mailbox, |_| {
            entered
                .take()
                .expect("only one write may start")
                .send(())
                .unwrap();
            let _ = blocked.recv();
            anyhow::ensure!(!mailbox.retirement.is_retired(), BLOCKED_IO);
            Ok(())
        });
        assert!(result.is_err());
        mailbox.retire();
        finished.send_replace(true);
    });
    let active = cache.record(history.clone()).unwrap();
    timeout_at(Instant::now() + STOP_TIMEOUT, started)
        .await
        .unwrap()
        .unwrap();
    let pending = cache.record(history.clone()).unwrap();
    let guard = timeout_at(Instant::now() + DEADLINE, book.lock())
        .await
        .unwrap();
    assert_eq!(guard.history(), history);
    drop(guard);
    cache.retire();
    assert!(pending.await.is_err());
    cache.wait_closed(Instant::now() + DEADLINE).await;
    assert!(!*cache.closed.borrow());
    assert!(cache.record(history).is_none());
    release.send(()).unwrap();
    assert!(
        timeout_at(Instant::now() + STOP_TIMEOUT, active)
            .await
            .unwrap()
            .is_err()
    );
    cache.wait_closed(Instant::now() + STOP_TIMEOUT).await;
    assert!(*cache.closed.borrow());
}

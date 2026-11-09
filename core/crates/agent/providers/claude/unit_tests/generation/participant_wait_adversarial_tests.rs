use super::managed_mutation_support::ManagedFixture;
use std::sync::{Arc, mpsc};
use std::time::Duration;

const NOT_ARCHIVED: bool = false;
const TEST_TIMEOUT: Duration = Duration::from_secs(2);

#[test]
fn lifecycle_contention_waits_without_creating_generation() {
    let fixture = Arc::new(ManagedFixture::new(NOT_ARCHIVED, None));
    let holder = fixture
        .paths
        .try_acquire_session_lifecycle_lock(fixture.session_id)
        .unwrap()
        .unwrap();
    let (observed_tx, observed_rx) = mpsc::channel();
    let participant = Arc::clone(&fixture);
    let waiter = std::thread::spawn(move || {
        observed_tx.send(()).unwrap();
        participant.begin()
    });

    observed_rx.recv_timeout(TEST_TIMEOUT).unwrap();
    assert!(!fixture.store.path().exists());
    drop(holder);
    waiter.join().unwrap().unwrap();
    assert!(fixture.store.path().exists());
}

#[test]
fn waited_generation_mutation_revalidates_the_pending_reboot_intent() {
    let fixture = Arc::new(ManagedFixture::new(NOT_ARCHIVED, None));
    let holder = fixture
        .paths
        .try_acquire_session_lifecycle_lock(fixture.session_id)
        .unwrap()
        .unwrap();
    let (observed_tx, observed_rx) = mpsc::channel();
    let participant = Arc::clone(&fixture);
    let waiter = std::thread::spawn(move || {
        observed_tx.send(()).unwrap();
        participant.begin()
    });

    observed_rx.recv_timeout(TEST_TIMEOUT).unwrap();
    crate::paths::atomic_write_json(&fixture.intent(), &serde_json::Value::Null).unwrap();
    drop(holder);

    assert!(waiter.join().unwrap().is_err());
    assert!(!fixture.store.path().exists());
}

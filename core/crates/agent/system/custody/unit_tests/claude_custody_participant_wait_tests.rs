use super::fence_test_support as support;
use super::*;
use std::sync::{Arc, mpsc};
use std::time::Duration;

const NOT_ARCHIVED: bool = false;
const TEST_TIMEOUT: Duration = Duration::from_secs(2);

#[test]
fn waited_custody_mutation_commits_after_lifecycle_release() {
    let fixture = Arc::new(support::Fixture::new(NOT_ARCHIVED, None));
    let holder = crate::paths::test_support::hold_session_lifecycle_lock(
        &fixture.paths,
        fixture.terminal_id,
    )
    .unwrap();
    let (observed_rx, waiter) = spawn_waiter(&fixture);

    observed_rx.recv_timeout(TEST_TIMEOUT).unwrap();
    assert!(!fixture.ledger.exists());
    drop(holder);
    waiter.join().unwrap().unwrap();
    assert!(fixture.ledger.exists());
}

#[test]
fn waited_custody_mutation_revalidates_the_session_manifest() {
    let fixture = Arc::new(support::Fixture::new(NOT_ARCHIVED, None));
    let holder = crate::paths::test_support::hold_session_lifecycle_lock(
        &fixture.paths,
        fixture.terminal_id,
    )
    .unwrap();
    let (observed_rx, waiter) = spawn_waiter(&fixture);

    observed_rx.recv_timeout(TEST_TIMEOUT).unwrap();
    archive_manifest(&fixture);
    drop(holder);

    assert!(waiter.join().unwrap().is_err());
    assert!(!fixture.ledger.exists());
}

fn spawn_waiter(
    fixture: &Arc<support::Fixture>,
) -> (
    mpsc::Receiver<()>,
    std::thread::JoinHandle<anyhow::Result<()>>,
) {
    let (observed_tx, observed_rx) = mpsc::channel();
    let participant = Arc::clone(fixture);
    let waiter = std::thread::spawn(move || {
        crate::paths::test_support::set_private_file_wait_contention_hook(move || {
            observed_tx.send(()).unwrap();
        });
        arm(&participant.ledger, &participant.spec)
    });
    (observed_rx, waiter)
}

fn archive_manifest(fixture: &support::Fixture) {
    let path = fixture.paths.manifest(fixture.terminal_id);
    let mut manifest: crate::model::TerminalManifest = crate::paths::read_json(&path).unwrap();
    manifest.archived = true;
    crate::paths::atomic_write_json(&path, &manifest).unwrap();
}

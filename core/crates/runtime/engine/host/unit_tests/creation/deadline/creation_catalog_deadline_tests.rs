use crate::host::tests::*;
use super::*;

const CREATION_CATALOG_BLOCKING_FAILURE: &str = "catalog operation settled before blocking";

#[tokio::test]
async fn creation_catalog_wait_obeys_the_absolute_deadline_and_fences_late_commit() {
    let directory = tempdir().unwrap();
    let host = Host::open(HostConfig::new(directory.path())).unwrap();
    let manifest = fixture_manifest(directory.path());
    host.catalog_insert(&manifest).await.unwrap();
    let mut replacement = manifest.clone();
    replacement.recovery_state = RecoveryState::Uncertain;
    let catalog = Arc::clone(&host.catalog);
    let (locked, locked_ready) = std::sync::mpsc::channel();
    let (release, released) = std::sync::mpsc::channel();
    let blocker = std::thread::spawn(move || {
        let _guard = lock_recover(&catalog);
        locked.send(()).unwrap();
        released.recv_timeout(HOST_BLOCKING_TEST_TIMEOUT).unwrap();
    });
    locked_ready
        .recv_timeout(HOST_BLOCKING_TEST_TIMEOUT)
        .unwrap();
    let (entered, mut operation_entries) = tokio::sync::mpsc::unbounded_channel();
    let (completed, mut operation_completions) = tokio::sync::mpsc::unbounded_channel();
    *lock_recover(&host.catalog_test_probes.operations) = Some(entered);
    *lock_recover(&host.catalog_test_probes.operation_completions) = Some(completed);
    let operation = host.catalog_upsert_before(
        &replacement,
        HostOperationDeadline::after(CREATE_DEADLINE_TEST_TIMEOUT),
    );
    tokio::pin!(operation);
    tokio::select! {
        entered = operation_entries.recv() => assert!(entered.is_some()),
        result = &mut operation => panic!("{CREATION_CATALOG_BLOCKING_FAILURE}: {result:?}"),
    }

    let error = tokio::time::timeout(HOST_BLOCKING_TEST_TIMEOUT, &mut operation)
        .await
        .unwrap()
        .unwrap_err();
    assert!(error.is::<CreationPublicationDeadlineExpired>());
    release.send(()).unwrap();
    blocker.join().unwrap();
    tokio::time::timeout(HOST_BLOCKING_TEST_TIMEOUT, operation_completions.recv())
        .await
        .unwrap()
        .unwrap();
    let stored = host
        .catalog_get(manifest.terminal_id)
        .await
        .unwrap()
        .unwrap();
    assert!(same_manifest(&stored, &manifest));
}

#[tokio::test]
async fn creation_catalog_timeout_reports_commit_owned_before_late_commit() {
    let directory = tempdir().unwrap();
    let host = Host::open(HostConfig::new(directory.path())).unwrap();
    let manifest = fixture_manifest(directory.path());
    host.catalog_insert(&manifest).await.unwrap();
    let mut replacement = manifest.clone();
    replacement.recovery_state = RecoveryState::Uncertain;
    let (owned, mut ownership) = tokio::sync::mpsc::unbounded_channel();
    let (release, released) = std::sync::mpsc::channel();
    *lock_recover(&host.catalog_test_probes.creation_commit_owned) = Some(Box::new(move || {
        owned.send(()).unwrap();
        released.recv_timeout(HOST_BLOCKING_TEST_TIMEOUT).unwrap();
    }));
    let (completed, mut completions) = tokio::sync::mpsc::unbounded_channel();
    *lock_recover(&host.catalog_test_probes.operation_completions) = Some(completed);
    let operation = host.catalog_upsert_before(
        &replacement,
        HostOperationDeadline::after(CREATE_DEADLINE_TEST_TIMEOUT),
    );
    tokio::pin!(operation);
    tokio::select! {
        value = ownership.recv() => assert!(value.is_some()),
        result = &mut operation => panic!("{CREATION_CATALOG_BLOCKING_FAILURE}: {result:?}"),
    }

    let error = tokio::time::timeout(HOST_BLOCKING_TEST_TIMEOUT, &mut operation)
        .await
        .unwrap()
        .unwrap_err();
    assert!(error.is::<CreationPublicationCommitInFlight>());
    release.send(()).unwrap();
    tokio::time::timeout(HOST_BLOCKING_TEST_TIMEOUT, completions.recv())
        .await
        .unwrap()
        .unwrap();
    let stored = host
        .catalog_get(manifest.terminal_id)
        .await
        .unwrap()
        .unwrap();
    assert!(same_manifest(&stored, &replacement));
}

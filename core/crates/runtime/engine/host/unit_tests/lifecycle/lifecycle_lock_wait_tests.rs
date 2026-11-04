use crate::host::tests::*;
use super::*;

const LIFECYCLE_LOCK_RELEASE_DELAY: Duration = Duration::from_millis(40);
const LIFECYCLE_LOCK_ACQUIRE_TIMEOUT: Duration = Duration::from_secs(1);
const LIFECYCLE_LOCK_TIMEOUT_BUDGET: Duration = Duration::from_millis(30);
const LIFECYCLE_LOCK_RETRY: Duration = Duration::from_millis(5);
const LIFECYCLE_LOCK_WAIT_SOCKET_NAME: &str = "lifecycle-lock-wait.sock";

#[tokio::test]
async fn product_lifecycle_waits_for_publication_lock_release() {
    let (_directory, host, selection, runtime) = product_lifecycle_fixture().await;
    let lifecycle = held_lifecycle_lock(&host, selection.terminal_id);
    let release = release_later(lifecycle);

    let (_, fence) = tokio::time::timeout(
        LIFECYCLE_LOCK_ACQUIRE_TIMEOUT,
        host.acquire_lifecycle_mutation_fence(&selection),
    )
    .await
    .unwrap()
    .unwrap();

    assert!(matches!(
        fence.runtime,
        LifecycleRuntimeFence::Selected(ref observed)
            if observed.runtime_epoch == runtime.runtime_epoch
    ));
    release.join().unwrap();
}

#[tokio::test]
async fn completed_lifecycle_waits_for_publication_lock_release() {
    let (_directory, host, selection, _runtime) = product_lifecycle_fixture().await;
    let (_, fence) = host
        .acquire_lifecycle_mutation_fence(&selection)
        .await
        .unwrap();
    let lifecycle = held_lifecycle_lock(&host, selection.terminal_id);
    let release = release_later(lifecycle);

    tokio::time::timeout(
        LIFECYCLE_LOCK_ACQUIRE_TIMEOUT,
        host.verify_completed_lifecycle_mutation(selection.terminal_id, &fence),
    )
    .await
    .unwrap()
    .unwrap();
    release.join().unwrap();
}

#[tokio::test]
async fn retained_lifecycle_fence_waits_for_publication_lock_release() {
    let (_directory, host, selection, _runtime) = product_lifecycle_fixture().await;
    let (_, fence) = host
        .acquire_lifecycle_mutation_fence(&selection)
        .await
        .unwrap();
    let lifecycle = held_lifecycle_lock(&host, selection.terminal_id);
    let release = release_later(lifecycle);

    let retained = tokio::time::timeout(
        LIFECYCLE_LOCK_ACQUIRE_TIMEOUT,
        host.acquire_retained_lifecycle_fence(selection.terminal_id, &fence),
    )
    .await
    .unwrap()
    .unwrap();
    drop(retained);
    release.join().unwrap();
}

#[tokio::test]
async fn lifecycle_wait_revalidates_epoch_after_lock_release() {
    let (_directory, host, selection, runtime) = product_lifecycle_fixture().await;
    let mut replacement = runtime.clone();
    replacement.runtime_epoch = Uuid::new_v4();
    replacement.worker_id = Uuid::new_v4();
    let runtime_path = host.paths.runtime_record(selection.terminal_id);
    let lifecycle = held_lifecycle_lock(&host, selection.terminal_id);
    let replacement_for_thread = replacement.clone();
    let release = std::thread::spawn({
        let runtime_path = runtime_path.clone();
        move || {
            std::thread::sleep(LIFECYCLE_LOCK_RELEASE_DELAY);
            atomic_write_json(&runtime_path, &replacement_for_thread).unwrap();
            drop(lifecycle);
        }
    });

    let result = tokio::time::timeout(
        LIFECYCLE_LOCK_ACQUIRE_TIMEOUT,
        host.acquire_lifecycle_mutation_fence(&selection),
    )
    .await
    .unwrap();
    let error = match result {
        Ok(_) => panic!("stale runtime unexpectedly acquired a lifecycle fence"),
        Err(error) => error,
    };

    assert_eq!(error.code, crate::protocol::error_code::CONFLICT);
    assert_eq!(
        read_json::<RuntimeRecord>(&runtime_path).unwrap(),
        replacement
    );
    let manifest: TerminalManifest = read_json(&host.paths.manifest(selection.terminal_id)).unwrap();
    assert_eq!(manifest.terminal_id, selection.terminal_id);
    assert!(!manifest.archived);
    release.join().unwrap();
}

#[tokio::test]
async fn lifecycle_lock_wait_still_fails_closed_after_budget() {
    let (_directory, host, selection, _runtime) = product_lifecycle_fixture().await;
    let lifecycle = held_lifecycle_lock(&host, selection.terminal_id);

    let error = host
        .wait_for_resume_lifecycle_lock_test(
            selection.terminal_id,
            LIFECYCLE_LOCK_TIMEOUT_BUDGET,
            LIFECYCLE_LOCK_RETRY,
        )
        .await
        .err()
        .unwrap();

    drop(lifecycle);
    assert_eq!(error.code, crate::protocol::error_code::CONFLICT);
    assert_eq!(error.message, LIFECYCLE_MUTATION_BUSY);
}

async fn product_lifecycle_fixture() -> (tempfile::TempDir, Host, TerminalLifecycleSelection, RuntimeRecord)
{
    let directory = tempdir().unwrap();
    let host = Host::open(HostConfig::new(directory.path())).unwrap();
    let manifest = fixture_manifest(directory.path());
    let runtime = host_runtime_record(
        &host,
        manifest.terminal_id,
        directory.path().join(LIFECYCLE_LOCK_WAIT_SOCKET_NAME),
    );
    persist_host_session(&host, &manifest, &runtime);
    let selection = TerminalLifecycleSelection::at_runtime_epoch(manifest.terminal_id, runtime.runtime_epoch);
    (directory, host, selection, runtime)
}

fn held_lifecycle_lock(
    host: &Host,
    terminal_id: Uuid,
) -> crate::paths::private_file_lock::PrivateFileLock {
    host.paths
        .try_acquire_session_lifecycle_lock(terminal_id)
        .unwrap()
        .unwrap()
}

fn release_later(
    lifecycle: crate::paths::private_file_lock::PrivateFileLock,
) -> std::thread::JoinHandle<()> {
    std::thread::spawn(move || {
        std::thread::sleep(LIFECYCLE_LOCK_RELEASE_DELAY);
        drop(lifecycle);
    })
}

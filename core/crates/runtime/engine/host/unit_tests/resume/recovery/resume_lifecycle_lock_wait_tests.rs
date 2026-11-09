use crate::host::tests::*;
use super::*;

const RESUME_LOCK_RELEASE_DELAY: Duration = Duration::from_millis(40);
const RESUME_LOCK_ACQUIRE_TIMEOUT: Duration = Duration::from_secs(1);
const RESUME_LOCK_TIMEOUT_BUDGET: Duration = Duration::from_millis(30);
const RESUME_LOCK_RETRY: Duration = Duration::from_millis(5);

#[tokio::test]
async fn resume_waits_for_predecessor_lifecycle_release() {
    let directory = tempdir().unwrap();
    let host = Host::open(HostConfig::new(directory.path())).unwrap();
    let manifest = fixture_manifest(directory.path());
    seed_recovery_manifest(&host, &manifest).await;
    let lifecycle = host
        .paths
        .try_acquire_session_lifecycle_lock(manifest.terminal_id)
        .unwrap()
        .unwrap();
    let release = std::thread::spawn(move || {
        std::thread::sleep(RESUME_LOCK_RELEASE_DELAY);
        drop(lifecycle);
    });

    let acquired = host
        .wait_for_resume_lifecycle_lock_test(
            manifest.terminal_id,
            RESUME_LOCK_ACQUIRE_TIMEOUT,
            RESUME_LOCK_RETRY,
        )
        .await
        .unwrap();

    drop(acquired);
    release.join().unwrap();
}

#[tokio::test]
async fn resume_lifecycle_wait_fails_closed_after_budget() {
    let directory = tempdir().unwrap();
    let host = Host::open(HostConfig::new(directory.path())).unwrap();
    let manifest = fixture_manifest(directory.path());
    seed_recovery_manifest(&host, &manifest).await;
    let lifecycle = host
        .paths
        .try_acquire_session_lifecycle_lock(manifest.terminal_id)
        .unwrap()
        .unwrap();

    let result = host
        .wait_for_resume_lifecycle_lock_test(
            manifest.terminal_id,
            RESUME_LOCK_TIMEOUT_BUDGET,
            RESUME_LOCK_RETRY,
        )
        .await;

    drop(lifecycle);
    let Err(error) = result else {
        panic!("resume unexpectedly acquired the held lifecycle lock");
    };
    assert_eq!(error.code, crate::protocol::error_code::CONFLICT);
    assert_eq!(error.message, LIFECYCLE_MUTATION_BUSY);
}

use super::*;
use yolo_runtime::paths::private_file::PrivateFileEntry;

const SNAPSHOT_LOCK_SESSION: &str = "power-loss-guardian-snapshot-lock";
const SNAPSHOT_LOCK_FAILURE: &str = "injected custody snapshot observation failure";

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn guardian_snapshot_excludes_same_directory_writers_and_releases_the_lock() {
    let harness = Harness::start().await;
    let terminal_id = harness
        .create_cat(SNAPSHOT_LOCK_SESSION)
        .await
        .manifest
        .terminal_id;
    let paths = harness.paths();
    let writer = PrivateFileEntry::open(&paths.runtime_record(terminal_id)).unwrap();
    let mut observed_lock = false;
    let observation =
        ActiveCustodyObservation::read_observing_session_lock(&paths, terminal_id, || {
            observed_lock = true;
            assert!(
                writer
                    .lock_parent_exclusive_wait(Duration::ZERO, PROCESS_POLL_INTERVAL)
                    .is_err()
            );
            Ok(())
        })
        .unwrap();
    assert!(observed_lock);
    observation.confirm_unchanged(&paths, terminal_id).unwrap();
    let lock = writer
        .lock_parent_exclusive_wait(PROCESS_EXIT_TIMEOUT, PROCESS_POLL_INTERVAL)
        .unwrap();
    drop(lock);
    drop(observation);

    let error = ActiveCustodyObservation::read_observing_session_lock(&paths, terminal_id, || {
        anyhow::bail!(SNAPSHOT_LOCK_FAILURE)
    })
    .unwrap_err();
    assert_eq!(error.to_string(), SNAPSHOT_LOCK_FAILURE);
    let lock = writer
        .lock_parent_exclusive_wait(PROCESS_EXIT_TIMEOUT, PROCESS_POLL_INTERVAL)
        .unwrap();
    drop(lock);
    ActiveCustodyObservation::read(&paths, terminal_id).unwrap();
}

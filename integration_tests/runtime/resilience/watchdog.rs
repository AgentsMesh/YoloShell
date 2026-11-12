use super::*;
use yolo_process_test_support::ProcessIdentityObservation;

const WATCHDOG_RECOVERY_TIMEOUT: Duration = Duration::from_secs(15);
const WATCHDOG_POLL_INTERVAL: Duration = Duration::from_millis(50);
const NON_RECOVERY_WINDOW: Duration = Duration::from_secs(5);
const CORRUPT_RUNTIME_BYTES: &[u8] = b"invalid-runtime-record";

#[path = "watchdog/recreation.rs"]
mod recreation;

struct RuntimeFileRestore {
    path: PathBuf,
    bytes: Vec<u8>,
}

impl RuntimeFileRestore {
    fn corrupt(path: PathBuf) -> Self {
        let bytes = std::fs::read(&path).unwrap();
        std::fs::write(&path, CORRUPT_RUNTIME_BYTES).unwrap();
        Self { path, bytes }
    }
}

impl Drop for RuntimeFileRestore {
    fn drop(&mut self) {
        let _ = std::fs::write(&self.path, &self.bytes);
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn detached_watchdog_requires_shell_recreation_and_isolates_a_corrupt_session() {
    let harness = Harness::start().await;
    let missing_exit = harness.create_cat("watchdog-missing-exit").await;
    let matching_crash = harness.create_cat("watchdog-worker-crash").await;
    let survivor = harness.create_cat("watchdog-survivor").await;
    let corrupt = harness.create_cat("watchdog-corrupt-peer").await;
    let paths = harness.paths();
    let missing_id = missing_exit.manifest.terminal_id;
    let crash_id = matching_crash.manifest.terminal_id;
    let survivor_id = survivor.manifest.terminal_id;
    let corrupt_id = corrupt.manifest.terminal_id;
    let missing_record = read_json::<RuntimeRecord>(&paths.runtime_record(missing_id)).unwrap();
    let crash_record = read_json::<RuntimeRecord>(&paths.runtime_record(crash_id)).unwrap();
    let survivor_record = read_json::<RuntimeRecord>(&paths.runtime_record(survivor_id)).unwrap();
    let host_status = harness.client().status().await.unwrap();
    let _restore = RuntimeFileRestore::corrupt(paths.runtime_record(corrupt_id));
    let _ = std::fs::remove_file(paths.exit_record(missing_record.terminal_id));
    let crash_journal = yolo_runtime::journal::Journal::inspect_retained_evidence(
        yolo_runtime::paths::private_file::PrivateFileEntry::open(&paths.journal(crash_id))
            .unwrap(),
        crash_record.runtime_epoch,
    )
    .unwrap();
    yolo_runtime::paths::atomic_write_json(
        &paths.exit_record(crash_record.terminal_id),
        &yolo_runtime::ExitRecord {
            runtime_epoch: crash_record.runtime_epoch,
            code: None,
            signal: None,
            reason: yolo_runtime::ExitReason::WorkerCrash,
            observed_at_ms: yolo_runtime::model::now_ms(),
            last_output_sequence: crash_journal.last_sequence(),
        },
    )
    .unwrap();
    assert!(
        runtime_worker_identity(&missing_record)
            .signal(libc::SIGKILL)
            .unwrap()
    );
    assert!(
        runtime_worker_identity(&crash_record)
            .signal(libc::SIGKILL)
            .unwrap()
    );
    crash_journal.verify_current().unwrap();

    let restored_missing = recreation::confirm_and_recreate(&harness, &missing_record).await;
    let restored_crash = recreation::confirm_and_recreate(&harness, &crash_record).await;
    for restored in [&restored_missing, &restored_crash] {
        let trusted = harness
            .client()
            .get_session(restored.terminal_id)
            .await
            .unwrap();
        harness.track_view(&trusted);
    }
    assert_restored_identity(&restored_missing, &missing_record);
    assert_restored_identity(&restored_crash, &crash_record);
    for restored in [&restored_missing, &restored_crash] {
        assert_eq!(restored.boot_id, host_status.boot_id);
        assert_eq!(
            restored.controller_generation,
            host_status.controller_generation
        );
    }
    let unchanged_survivor =
        read_json::<RuntimeRecord>(&paths.runtime_record(survivor_record.terminal_id)).unwrap();
    assert_eq!(
        unchanged_survivor.runtime_epoch,
        survivor_record.runtime_epoch
    );
    assert_eq!(unchanged_survivor.worker_pid, survivor_record.worker_pid);
    assert_eq!(
        std::fs::read(paths.runtime_record(corrupt_id)).unwrap(),
        CORRUPT_RUNTIME_BYTES
    );
    wait_for_identity_gone(runtime_worker_identity(&missing_record)).await;
    wait_for_identity_gone(runtime_worker_identity(&crash_record)).await;
    discard_incomplete_coverage_profile(missing_record.worker_pid);
    discard_incomplete_coverage_profile(crash_record.worker_pid);
    wait_for_identity_gone(runtime_child_identity(&missing_record)).await;
    wait_for_identity_gone(runtime_child_identity(&crash_record)).await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn watchdog_never_recreates_an_explicitly_terminated_session() {
    let harness = Harness::start().await;
    let created = harness.create_cat("watchdog-user-terminate").await;
    let terminal_id = created.manifest.terminal_id;
    let record = read_json::<RuntimeRecord>(&harness.paths().runtime_record(terminal_id)).unwrap();
    harness
        .client()
        .terminate(record.terminal_id, true)
        .await
        .unwrap();
    wait_for_exit(&harness.client(), record.terminal_id).await;
    wait_for_identity_gone(runtime_worker_identity(&record)).await;
    tokio::time::sleep(NON_RECOVERY_WINDOW).await;
    let persisted =
        read_json::<RuntimeRecord>(&harness.paths().runtime_record(record.terminal_id)).unwrap();
    let exit =
        read_json::<yolo_runtime::ExitRecord>(&harness.paths().exit_record(record.terminal_id))
            .unwrap();
    assert_eq!(persisted.runtime_epoch, record.runtime_epoch);
    assert_eq!(exit.runtime_epoch, record.runtime_epoch);
    assert_eq!(exit.reason, yolo_runtime::ExitReason::UserTerminate);
    assert_eq!(
        runtime_worker_identity(&record).observe(),
        ProcessIdentityObservation::Gone
    );
}

fn assert_restored_identity(restored: &RuntimeRecord, previous: &RuntimeRecord) {
    assert_eq!(restored.terminal_id, previous.terminal_id);
    assert_ne!(restored.runtime_epoch, previous.runtime_epoch);
    assert_ne!(
        runtime_worker_identity(restored),
        runtime_worker_identity(previous)
    );
    assert!(runtime_worker_identity(restored).matches());
    assert!(runtime_child_identity(restored).matches());
}

fn runtime_worker_identity(record: &RuntimeRecord) -> ProcessIdentity {
    ProcessIdentity {
        pid: record.worker_pid,
        start_time: record.worker_start_time,
    }
}

fn runtime_child_identity(record: &RuntimeRecord) -> ProcessIdentity {
    ProcessIdentity {
        pid: record.child_pid,
        start_time: record.child_start_time,
    }
}

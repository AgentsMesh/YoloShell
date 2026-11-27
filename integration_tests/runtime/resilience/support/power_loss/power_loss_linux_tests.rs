use super::*;
use yolo_process_test_support::linux_containment::{LinuxContainmentRecord, record_path};

const CGROUP_EVENTS: &str = "cgroup.events";
const FROZEN_EVENT: &str = "frozen 1";

#[tokio::test]
async fn competing_capture_cannot_obtain_rollback_authority_before_freeze_is_written() {
    let directory = tempfile::tempdir().unwrap();
    let paths = Paths::new(directory.path());
    let worker = TestProcess::spawn();
    let child = TestProcess::spawn();
    let record = fixture_record(&paths, worker.identity, &child);
    let bound = bind_record(&paths, record.clone());
    let (_first_tree, mut first) = bound.capture_linux_power_loss_tree().unwrap();
    let (_second_tree, mut second) = bound.capture_linux_power_loss_tree().unwrap();
    first.begin_freeze().unwrap();
    assert!(!is_frozen(&paths, &record));
    assert!(second.begin_freeze().is_err());
    drop(first);
    second.begin_freeze().unwrap();
    assert!(worker.identity.matches());
    assert!(child.identity.matches());
}

#[tokio::test]
async fn second_transaction_cannot_thaw_the_first_transactions_freeze() {
    let directory = tempfile::tempdir().unwrap();
    let paths = Paths::new(directory.path());
    let worker = TestProcess::spawn();
    let child = TestProcess::spawn();
    let record = fixture_record(&paths, worker.identity, &child);
    let bound = bind_record(&paths, record.clone());
    let records = std::slice::from_ref(&bound);
    let first = prepare_bound_power_loss(
        &paths,
        records,
        GuardianExpectation::ExplicitlyAbsentFixture,
    )
    .await;
    let mut second = barrier::PowerLossFreezeBarrier::capture(&paths, records, Vec::new())
        .await
        .unwrap();
    assert!(second.freeze_all(&paths).await.is_err());
    drop(second);
    assert!(is_frozen(&paths, &record));
    assert!(worker.is_stably_stopped());
    drop(first);
    wait_for_thaw(&paths, &record).await;
    assert!(worker.identity.matches());
    assert!(child.identity.matches());
}

#[tokio::test]
async fn containment_replacement_before_commit_preserves_targets_and_rolls_back_freeze() {
    let directory = tempfile::tempdir().unwrap();
    let paths = Paths::new(directory.path());
    let worker = TestProcess::spawn();
    let child = TestProcess::spawn();
    let record = fixture_record(&paths, worker.identity, &child);
    let bound = bind_record(&paths, record.clone());
    let records = std::slice::from_ref(&bound);
    let mut barrier = prepare_bound_power_loss(
        &paths,
        records,
        GuardianExpectation::ExplicitlyAbsentFixture,
    )
    .await;
    assert!(is_frozen(&paths, &record));
    let path = record_path(&paths.session_dir(record.terminal_id), record.runtime_epoch);
    let original: LinuxContainmentRecord = read_json(&path).unwrap();

    yolo_runtime::paths::atomic_write_json(&path, &original).unwrap();
    let result = barrier.kill_all(&paths, records).await;

    assert!(result.is_err());
    assert!(worker.identity.matches());
    assert!(child.identity.matches());
    assert!(is_frozen(&paths, &record));
    drop(barrier);
    wait_for_thaw(&paths, &record).await;
    assert!(worker.identity.matches());
    assert!(child.identity.matches());
    assert!(!paths.exit_record(record.terminal_id).exists());
}

#[tokio::test]
async fn cancellation_of_frozen_linux_runtime_restores_every_exact_participant() {
    let directory = tempfile::tempdir().unwrap();
    let paths = Paths::new(directory.path());
    let worker = TestProcess::spawn();
    let child = TestProcess::spawn();
    let record = fixture_record(&paths, worker.identity, &child);
    let bound = bind_record(&paths, record.clone());
    let barrier = prepare_bound_power_loss(
        &paths,
        &[bound],
        GuardianExpectation::ExplicitlyAbsentFixture,
    )
    .await;
    assert!(is_frozen(&paths, &record));
    let (started, ready) = tokio::sync::oneshot::channel();
    let task = tokio::spawn(async move {
        let _barrier = barrier;
        started.send(()).unwrap();
        std::future::pending::<()>().await;
    });
    ready.await.unwrap();
    task.abort();
    assert!(task.await.unwrap_err().is_cancelled());
    wait_for_thaw(&paths, &record).await;
    assert!(worker.identity.matches());
    assert!(!worker.is_stably_stopped());
    assert!(child.identity.matches());
    assert!(!paths.exit_record(record.terminal_id).exists());
}

#[tokio::test]
async fn failure_after_tree_kill_keeps_cleanup_committed_and_emits_no_exit_evidence() {
    let directory = tempfile::tempdir().unwrap();
    let paths = Paths::new(directory.path());
    let worker = TestProcess::spawn();
    let child = TestProcess::spawn();
    let record = fixture_record(&paths, worker.identity, &child);
    let bound = bind_record(&paths, record.clone());
    let records = std::slice::from_ref(&bound);
    let mut barrier = prepare_bound_power_loss(
        &paths,
        records,
        GuardianExpectation::ExplicitlyAbsentFixture,
    )
    .await;
    let result = barrier
        .kill_all_after_session_confirm_for_test(
            &paths,
            records,
            barrier::PowerLossCommitBoundary::DirectTarget,
            || {
                yolo_runtime::paths::atomic_write_json(
                    &paths.runtime_record(record.terminal_id),
                    &record,
                )
                .unwrap()
            },
        )
        .await;
    assert!(result.is_err());
    drop(barrier);
    wait_for_identity_gone(worker.identity).await;
    wait_for_identity_gone(child.identity).await;
    assert!(!paths.exit_record(record.terminal_id).exists());
}

fn is_frozen(paths: &Paths, runtime: &RuntimeRecord) -> bool {
    let path = record_path(
        &paths.session_dir(runtime.terminal_id),
        runtime.runtime_epoch,
    );
    let containment: LinuxContainmentRecord = read_json(&path).unwrap();
    std::fs::read_to_string(
        containment
            .delegated_root
            .join(containment.leaf_name)
            .join(CGROUP_EVENTS),
    )
    .unwrap()
    .lines()
    .any(|line| line == FROZEN_EVENT)
}

async fn wait_for_thaw(paths: &Paths, runtime: &RuntimeRecord) {
    const THAW_TIMEOUT: &str = "owned Linux fixture cgroup did not thaw";
    let deadline = tokio::time::Instant::now() + PROCESS_EXIT_TIMEOUT;
    while is_frozen(paths, runtime) {
        assert!(tokio::time::Instant::now() < deadline, "{THAW_TIMEOUT}");
        tokio::time::sleep(PROCESS_POLL_INTERVAL).await;
    }
}

use super::super::runtime_record_test_support::bind_record;
use super::*;
use yolo_runtime::model::{BootstrapDelivery, ExitReason, ExitRecord};

#[path = "power_loss_authority_commit_tests.rs"]
mod authority_commit_tests;
#[cfg(target_os = "linux")]
#[path = "power_loss_linux_tests.rs"]
mod linux_tests;

const TEST_PROCESS_EXECUTABLE: &str = "/bin/sleep";
const TEST_PROCESS_ARGUMENT: &str = "60";
const TEST_AUTH_TOKEN: &str = "power-loss-auth";
const TEST_CONTROLLER_GENERATION: u64 = 1;
const TEST_STARTED_AT_MS: i64 = i64::MIN;
const TEST_LAST_OUTPUT_SEQUENCE: u64 = u64::MIN;

#[path = "power_loss_fixture_process.rs"]
mod fixture_process;
use fixture_process::TestProcess;

#[tokio::test]
async fn worker_first_power_loss_removes_both_process_identities_without_exit_evidence() {
    let directory = tempfile::tempdir().unwrap();
    let paths = Paths::new(directory.path());
    let worker = TestProcess::spawn();
    let child = TestProcess::spawn();
    let record = fixture_record(&paths, worker.identity, &child);

    let bound = bind_record(&paths, record.clone());
    simulate_bound_power_loss(
        &paths,
        std::slice::from_ref(&bound),
        GuardianExpectation::ExplicitlyAbsentFixture,
    )
    .await;

    assert_runtime_identities_gone(&record);
    assert!(!paths.exit_record(record.terminal_id).try_exists().unwrap());
}

#[tokio::test]
async fn exit_evidence_after_worker_loss_fails_closed_and_is_preserved() {
    let directory = tempfile::tempdir().unwrap();
    let paths = Paths::new(directory.path());
    let worker = TestProcess::spawn();
    let child = TestProcess::spawn();
    let record = fixture_record(&paths, worker.identity, &child);
    let bound = bind_record(&paths, record.clone());
    let records = std::slice::from_ref(&bound);
    validation::validate_targets(&paths, records).unwrap();
    validation::assert_exit_evidence_absent(&paths, records);

    workers::signal(records);
    workers::wait_for_exit(records).await;
    assert!(!worker.identity.matches());
    assert!(child.identity.matches());
    let exit_path = paths.exit_record(record.terminal_id);
    write_exit(&exit_path, record.runtime_epoch);
    let evidence = std::fs::read(&exit_path).unwrap();

    assert!(
        std::panic::catch_unwind(|| validation::assert_exit_evidence_absent(&paths, records))
            .is_err()
    );
    assert_eq!(std::fs::read(&exit_path).unwrap(), evidence);
    assert!(child.identity.signal(libc::SIGKILL).unwrap());
    wait_for_identity_gone(child.identity).await;
    assert!(!child.identity.matches());
}

#[tokio::test]
async fn runtime_replacement_after_tree_confirm_fails_before_destructive_signal() {
    let directory = tempfile::tempdir().unwrap();
    let paths = Paths::new(directory.path());
    let worker = TestProcess::spawn();
    let child = TestProcess::spawn();
    let record = fixture_record(&paths, worker.identity, &child);
    let bound = bind_record(&paths, record.clone());
    let records = std::slice::from_ref(&bound);
    let runtime_worker = runtime_worker(&bound);
    let runtime_child = runtime_child(&bound);
    let guardians = guardians::capture(
        &paths,
        records,
        GuardianExpectation::ExplicitlyAbsentFixture,
    )
    .unwrap();
    let mut barrier = barrier::PowerLossFreezeBarrier::capture(&paths, records, guardians)
        .await
        .unwrap();
    barrier.freeze_all(&paths).await.unwrap();

    let result = barrier
        .kill_all_after_session_confirm_for_test(
            &paths,
            records,
            barrier::PowerLossCommitBoundary::RuntimeTree,
            || {
                yolo_runtime::paths::atomic_write_json(
                    &paths.runtime_record(record.terminal_id),
                    &record,
                )
                .unwrap();
            },
        )
        .await;

    assert!(result.is_err());
    assert!(runtime_worker.is_stably_stopped().unwrap());
    assert!(runtime_child.matches());
    barrier.ensure_all_frozen().await.unwrap();
    #[cfg(not(target_os = "linux"))]
    assert!(runtime_child.is_stably_stopped().unwrap());
}

fn fixture_record(paths: &Paths, worker: ProcessIdentity, child: &TestProcess) -> RuntimeRecord {
    let terminal_id = Uuid::new_v4();
    paths.ensure_session(terminal_id).unwrap();
    let record = RuntimeRecord {
        schema_version: yolo_runtime::constants::RUNTIME_RECORD_SCHEMA_VERSION,
        terminal_id: terminal_id,
        runtime_epoch: Uuid::new_v4(),
        worker_id: Uuid::new_v4(),
        worker_pid: worker.pid,
        worker_start_time: worker.start_time,
        child_pid: child.identity.pid,
        child_start_time: child.identity.start_time,
        boot_id: yolo_process_test_support::boot_id(),
        socket_path: paths.worker_socket(terminal_id).unwrap(),
        auth_token: TEST_AUTH_TOKEN.into(),
        controller_generation: TEST_CONTROLLER_GENERATION,
        started_at_ms: TEST_STARTED_AT_MS,
        command: TEST_PROCESS_EXECUTABLE.into(),
        args: vec![TEST_PROCESS_ARGUMENT.into()],
        cwd: paths.home().to_path_buf(),
        cols: yolo_runtime::model::MIN_TERMINAL_COLUMNS,
        rows: yolo_runtime::model::MIN_TERMINAL_ROWS,
        bootstrap_delivery: BootstrapDelivery::NotRequired,
    };
    yolo_runtime::paths::atomic_write_json(&paths.runtime_record(terminal_id), &record).unwrap();
    #[cfg(target_os = "linux")]
    child.bind_containment(paths, &record);
    record
}

fn write_exit(path: &Path, runtime_epoch: Uuid) {
    let exit = ExitRecord {
        runtime_epoch,
        code: None,
        signal: None,
        reason: ExitReason::HostReboot,
        observed_at_ms: TEST_STARTED_AT_MS,
        last_output_sequence: TEST_LAST_OUTPUT_SEQUENCE,
    };
    yolo_runtime::paths::atomic_write_json(path, &exit).unwrap();
}

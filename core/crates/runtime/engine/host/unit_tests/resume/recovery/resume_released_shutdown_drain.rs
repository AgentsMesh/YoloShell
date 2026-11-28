use super::*;

#[path = "../../../test_support/unit/secondary_custody/secondary_custody_runtime_child_test.rs"]
mod runtime_child;

#[tokio::test]
async fn withdrawn_terminal_listener_keeps_the_complete_worker_shutdown_budget() {
    let directory = tempdir().unwrap();
    let host = Host::open(HostConfig::new(directory.path())).unwrap();
    let mut worker = runtime_child::TestRuntimeChild::spawn();
    let mut record = host_runtime_record(&host, Uuid::new_v4(), directory.path().join(SOCKET_NAME));
    record.worker_pid = worker.identity().pid;
    record.worker_start_time = worker.identity().start_time;
    let child = reaped_released_exit_child_identity();
    record.child_pid = child.pid;
    record.child_start_time = child.start_time;
    let mut manifest = completed_test_manifest(&host, record.terminal_id);
    persist_host_session(&host, &manifest, &record);
    atomic_write_json(
        &host.paths.exit_record(record.terminal_id),
        &ExitRecord {
            runtime_epoch: record.runtime_epoch,
            code: Some(EXIT_CODE),
            signal: None,
            reason: ExitReason::UserTerminate,
            observed_at_ms: EXIT_OBSERVED_AT_MS,
            last_output_sequence: EXIT_OUTPUT_SEQUENCE,
        },
    )
    .unwrap();
    let _lease = host.secondary_custody.install_test_retired_lease(
        &host,
        &record,
        secondary_custody::TestRetiredCustodyState::Released,
    );
    let delay = DEFERRED_TERMINATION_SETTLEMENT_TIMEOUT.saturating_add(PROCESS_EXIT_POLL_INTERVAL);
    assert!(delay < crate::session_timing::WORKER_SHUTDOWN_SETTLEMENT_TIMEOUT);
    let started = tokio::time::Instant::now();
    let waiting = host.settle_disconnected_previous_runtime(&mut manifest, &record);
    let settle = async {
        tokio::time::sleep(delay).await;
        worker.settle();
    };
    let (result, ()) = tokio::join!(waiting, settle);
    result.unwrap();
    assert!(started.elapsed() >= delay);
}

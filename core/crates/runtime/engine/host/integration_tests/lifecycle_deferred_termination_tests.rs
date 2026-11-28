use crate::host::tests::*;
const DEFERRED_TERMINATION_DETAIL: &str = "fixture cleanup was deferred";
const OTHER_TERMINATION_DETAIL: &str = "fixture cleanup failed";
const DEFERRED_RUNTIME_SOCKET_NAME: &str = "deferred-runtime.sock";
const DEFERRED_SETTLEMENT_REPLAY_COUNT: usize = 2;

fn deferred_error() -> ErrorBody {
    ErrorBody::new(
        crate::protocol::error_code::CLEANUP_DEFERRED,
        DEFERRED_TERMINATION_DETAIL,
        false,
    )
}

fn deferred_fixture() -> (tempfile::TempDir, Host, ArchiveTestProcess, RuntimeRecord) {
    let directory = tempdir().unwrap();
    let host = Host::open(HostConfig::new(directory.path())).unwrap();
    let terminal_id = Uuid::new_v4();
    let worker = ArchiveTestProcess::spawn();
    let mut runtime = lifecycle_convergence_dead_runtime(&host_runtime_record(
        &host,
        terminal_id,
        directory.path().join(DEFERRED_RUNTIME_SOCKET_NAME),
    ));
    (runtime.worker_pid, runtime.worker_start_time) = worker.identity();
    persist_archive_runtime(&host, &runtime);
    persist_archive_exit(&host, &runtime, runtime.runtime_epoch);
    (directory, host, worker, runtime)
}

#[tokio::test]
async fn exact_durable_deferred_settlement_is_idempotently_acknowledged() {
    let (_directory, host, _live_worker, runtime) = deferred_fixture();

    for _ in 0..DEFERRED_SETTLEMENT_REPLAY_COUNT {
        host.settle_deferred_termination_with_timeout(
            &runtime,
            deferred_error(),
            ARCHIVE_TEST_TIMEOUT,
        )
        .await
        .unwrap();
    }
}

#[tokio::test]
async fn non_deferred_worker_failure_is_never_settled_as_success() {
    let (_directory, host, _live_worker, runtime) = deferred_fixture();
    let expected = ErrorBody::conflict(OTHER_TERMINATION_DETAIL);

    let error = host
        .settle_deferred_termination_with_timeout(&runtime, expected.clone(), ARCHIVE_TEST_TIMEOUT)
        .await
        .unwrap_err();

    assert_eq!(error, expected);
}

#[tokio::test]
async fn elapsed_deferred_settlement_budget_never_acknowledges_live_identities() {
    let directory = tempdir().unwrap();
    let host = Host::open(HostConfig::new(directory.path())).unwrap();
    let runtime = host_runtime_record(
        &host,
        Uuid::new_v4(),
        directory.path().join(DEFERRED_RUNTIME_SOCKET_NAME),
    );
    persist_archive_runtime(&host, &runtime);
    persist_archive_exit(&host, &runtime, runtime.runtime_epoch);
    let expected = deferred_error();

    let error = host
        .settle_deferred_termination_with_timeout(&runtime, expected.clone(), Duration::ZERO)
        .await
        .unwrap_err();

    assert_eq!(error, expected);
}

#[tokio::test(start_paused = true)]
async fn durable_evidence_published_at_the_deadline_is_observed() {
    let (_directory, host, _live_worker, mut runtime) = deferred_fixture();
    let mut child = ArchiveTestProcess::spawn();
    (runtime.child_pid, runtime.child_start_time) = child.identity();
    persist_archive_runtime(&host, &runtime);

    let mut settlement = Box::pin(host.settle_deferred_termination_with_timeout(
        &runtime,
        deferred_error(),
        ARCHIVE_TEST_TIMEOUT,
    ));
    std::future::poll_fn(|context| {
        assert!(std::future::Future::poll(settlement.as_mut(), context).is_pending());
        std::task::Poll::Ready(())
    })
    .await;

    child.stop();
    tokio::time::advance(ARCHIVE_TEST_TIMEOUT).await;

    settlement.await.unwrap();
}

#[tokio::test(start_paused = true)]
async fn production_deferred_budget_outlives_the_plain_process_exit_budget() {
    let (_directory, host, _live_worker, mut runtime) = deferred_fixture();
    let mut child = ArchiveTestProcess::spawn();
    (runtime.child_pid, runtime.child_start_time) = child.identity();
    persist_archive_runtime(&host, &runtime);

    let mut settlement = Box::pin(host.settle_deferred_termination(&runtime, deferred_error()));
    std::future::poll_fn(|context| {
        assert!(std::future::Future::poll(settlement.as_mut(), context).is_pending());
        std::task::Poll::Ready(())
    })
    .await;

    tokio::time::advance(PROCESS_EXIT_TIMEOUT).await;
    std::future::poll_fn(|context| {
        assert!(std::future::Future::poll(settlement.as_mut(), context).is_pending());
        std::task::Poll::Ready(())
    })
    .await;

    child.stop();
    tokio::time::advance(PROCESS_EXIT_POLL_INTERVAL).await;
    settlement.await.unwrap();
}

#[tokio::test]
async fn missing_exit_or_runtime_replacement_cannot_satisfy_deferred_settlement() {
    let (_directory, host, _live_worker, runtime) = deferred_fixture();
    std::fs::remove_file(host.paths.exit_record(runtime.terminal_id)).unwrap();
    let missing_exit = deferred_error();
    let error = host
        .settle_deferred_termination_with_timeout(
            &runtime,
            missing_exit.clone(),
            ARCHIVE_TEST_TIMEOUT,
        )
        .await
        .unwrap_err();
    assert_eq!(error, missing_exit);

    persist_archive_exit(&host, &runtime, runtime.runtime_epoch);
    let mut replacement = runtime.clone();
    replacement.runtime_epoch = Uuid::new_v4();
    replacement.worker_id = Uuid::new_v4();
    atomic_write_json(&host.paths.runtime_record(runtime.terminal_id), &replacement).unwrap();
    let expected = deferred_error();

    let error = host
        .settle_deferred_termination_with_timeout(&runtime, expected.clone(), ARCHIVE_TEST_TIMEOUT)
        .await
        .unwrap_err();

    assert_eq!(error, expected);
}

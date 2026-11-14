const RECONCILE_FIRST_SESSION_NAME: &str = "reconcile-first-boundary";

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn durable_tombstone_wins_when_termination_waits_for_reconciliation_lock() {
    let (mut harness, termination_root, reconcile_root) = start_barrier_harness().await;
    let terminal_id = prepare_session(&harness, RECONCILE_FIRST_SESSION_NAME).await;
    let paths = harness.paths();
    let before: RuntimeRecord = read_json(&paths.runtime_record(terminal_id)).unwrap();
    let reconcile_barrier = ReconcileBarrier::new(&reconcile_root, terminal_id).unwrap();
    let termination_barrier = TerminationBarrier::new(&termination_root, terminal_id).unwrap();
    reconcile_barrier.arm().unwrap();
    termination_barrier.arm().unwrap();

    let reconcile_client = harness.client();
    let reconciliation = tokio::spawn(async move { reconcile_client.reconcile().await });
    wait_for_barrier(|| reconcile_barrier.reached().unwrap()).await;

    let terminate_client = harness.client();
    let termination =
        tokio::spawn(async move { terminate_client.terminate(terminal_id, true).await });
    tokio::time::sleep(PROCESS_POLL_INTERVAL).await;
    assert!(!termination.is_finished());
    assert!(!termination_barrier.reached().unwrap());
    assert!(!paths.terminate_tombstone(terminal_id).exists());

    reconcile_barrier.release().unwrap();
    let result = timeout(PROCESS_EXIT_TIMEOUT, reconciliation)
        .await
        .unwrap()
        .unwrap()
        .unwrap();
    assert_eq!(result.reattached, 1);
    wait_for_barrier(|| termination_barrier.reached().unwrap()).await;
    assert_durable_boundary(&harness, &before).await;

    harness.kill_host_hard();
    await_disconnected(termination).await;
    kill_runtime_without_finalization(&before).await;
    harness.replace_host(spawn_host(harness.home.path())).await;

    assert_never_recreated(&harness, &before).await;
}

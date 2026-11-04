const TERMINATION_TEST_BARRIER_DIRECTORY: &str = "termination-test-barrier";
const RECONCILE_TEST_BARRIER_DIRECTORY: &str = "reconcile-test-barrier";
const TERMINATE_FIRST_SESSION_NAME: &str = "termination-first-boundary";

type TerminationBarrier = yolo_runtime::worker::termination_test_hook::TerminationTestBarrier;
type ReconcileBarrier = yolo_runtime::host::reconcile_test_hook::ReconcileTestBarrier;

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn durable_tombstone_wins_when_termination_reaches_the_barrier_first() {
    let (mut harness, barrier_root, _) = start_barrier_harness().await;
    let terminal_id = prepare_session(&harness, TERMINATE_FIRST_SESSION_NAME).await;
    let paths = harness.paths();
    let before: RuntimeRecord = read_json(&paths.runtime_record(terminal_id)).unwrap();
    let barrier = TerminationBarrier::new(&barrier_root, terminal_id).unwrap();
    barrier.arm().unwrap();

    let terminate_client = harness.client();
    let termination =
        tokio::spawn(async move { terminate_client.terminate(terminal_id, true).await });
    wait_for_barrier(|| barrier.reached().unwrap()).await;
    assert_durable_boundary(&harness, &before).await;

    let reconcile_client = harness.client();
    let reconciliation = tokio::spawn(async move { reconcile_client.reconcile().await });
    tokio::time::sleep(PROCESS_POLL_INTERVAL).await;
    assert!(!reconciliation.is_finished());

    harness.kill_host_hard();
    await_disconnected(termination).await;
    await_disconnected(reconciliation).await;
    kill_runtime_without_finalization(&before).await;
    harness.replace_host(spawn_host(harness.home.path())).await;

    assert_never_recreated(&harness, &before).await;
}

async fn start_barrier_harness() -> (Harness, PathBuf, PathBuf) {
    let home = tempfile::tempdir().unwrap();
    let termination_root = home.path().join(TERMINATION_TEST_BARRIER_DIRECTORY);
    let reconcile_root = home.path().join(RECONCILE_TEST_BARRIER_DIRECTORY);
    let host = spawn_barrier_host(home.path(), &termination_root, &reconcile_root);
    let harness = Harness::start_with_host(home, host).await;
    (harness, termination_root, reconcile_root)
}

fn spawn_barrier_host(home: &Path, termination_root: &Path, reconcile_root: &Path) -> SpawnedHost {
    let mut command = Command::new(runtime_binary());
    command
        .args(yolo_launch_contract::ProcessRole::Host.arguments())
        .args([cli::SERVE_COMMAND, cli::HOME_ARGUMENT])
        .arg(home)
        .env(TerminationBarrier::environment_variable(), termination_root)
        .env(ReconcileBarrier::environment_variable(), reconcile_root)
        .env(SHELL_ENV, TEST_SHELL)
        .stdin(Stdio::null())
        .stdout(Stdio::null());
    spawn_logged_host(home, &mut command)
}

async fn prepare_session(harness: &Harness, name: &str) -> Uuid {
    let terminal_id = harness.create_cat(name).await.manifest.terminal_id;
    let result = harness.client().reconcile().await.unwrap();
    assert_eq!(result.reattached, 1);
    let view = harness.client().get_session(terminal_id).await.unwrap();
    assert_eq!(
        view.manifest.recovery_state,
        yolo_runtime::RecoveryState::Reattached
    );
    terminal_id
}

async fn wait_for_barrier(mut reached: impl FnMut() -> bool) {
    let deadline = tokio::time::Instant::now() + PROCESS_EXIT_TIMEOUT;
    while !reached() {
        assert!(tokio::time::Instant::now() < deadline);
        tokio::time::sleep(PROCESS_POLL_INTERVAL).await;
    }
}

async fn assert_durable_boundary(harness: &Harness, before: &RuntimeRecord) {
    let paths = harness.paths();
    let stored: RuntimeRecord = read_json(&paths.runtime_record(before.terminal_id)).unwrap();
    assert_same_runtime(&stored, before);
    let tombstone: yolo_runtime::TerminationTombstone =
        read_json(&paths.terminate_tombstone(before.terminal_id)).unwrap();
    assert_eq!(
        tombstone.validate_for(before.terminal_id, before.runtime_epoch),
        Ok(())
    );
    assert!(tombstone.force);
    assert_identity_live(before.worker_pid, before.worker_start_time);
    assert_identity_live(before.child_pid, before.child_start_time);
    assert!(!paths.exit_record(before.terminal_id).exists());
    let view = harness
        .client()
        .get_session(before.terminal_id)
        .await
        .unwrap();
    assert_eq!(
        view.manifest.recovery_state,
        yolo_runtime::RecoveryState::Reattached
    );
    assert_eq!(view.runtime.unwrap().runtime_epoch, before.runtime_epoch);
}

async fn assert_never_recreated(harness: &Harness, before: &RuntimeRecord) {
    let paths = harness.paths();
    for _ in 0..RECONCILE_ATTEMPTS {
        let result = harness.client().reconcile().await.unwrap();
        assert_eq!(result.reattached, 0);
        assert_eq!(result.needs_recreate, 0);
    }
    let stored: RuntimeRecord = read_json(&paths.runtime_record(before.terminal_id)).unwrap();
    assert_same_runtime(&stored, before);
    assert_identity_gone(before.worker_pid, before.worker_start_time);
    assert_identity_gone(before.child_pid, before.child_start_time);
    assert!(!paths.exit_record(before.terminal_id).exists());
    assert!(!paths.worker_launch(before.terminal_id).exists());
    let tombstone: yolo_runtime::TerminationTombstone =
        read_json(&paths.terminate_tombstone(before.terminal_id)).unwrap();
    assert_eq!(
        tombstone.validate_for(before.terminal_id, before.runtime_epoch),
        Ok(())
    );
    let view = harness
        .client()
        .get_session(before.terminal_id)
        .await
        .unwrap();
    assert_eq!(
        view.manifest.recovery_state,
        yolo_runtime::RecoveryState::Reattached
    );
    let runtime = view.runtime.unwrap();
    assert_eq!(runtime.runtime_epoch, before.runtime_epoch);
    assert_eq!(runtime.status, TerminalRuntimeStatus::Lost);
    assert!(runtime.exit.is_none());
}

fn assert_same_runtime(actual: &RuntimeRecord, expected: &RuntimeRecord) {
    assert_eq!(actual.runtime_epoch, expected.runtime_epoch);
    assert_eq!(actual.worker_id, expected.worker_id);
    assert_eq!(actual.worker_pid, expected.worker_pid);
    assert_eq!(actual.worker_start_time, expected.worker_start_time);
    assert_eq!(actual.child_pid, expected.child_pid);
    assert_eq!(actual.child_start_time, expected.child_start_time);
    assert_eq!(actual.started_at_ms, expected.started_at_ms);
}

fn assert_identity_live(pid: u32, start_time: u64) {
    assert!(process::process_identity_matches(pid, start_time));
}

fn assert_identity_gone(pid: u32, start_time: u64) {
    assert!(!process::process_identity_matches(pid, start_time));
}

async fn await_disconnected<T>(task: tokio::task::JoinHandle<T>) {
    timeout(PROCESS_EXIT_TIMEOUT, task).await.unwrap().unwrap();
}

use super::*;

pub(super) async fn assert_get_only_completed_adoption(
    harness: &Harness,
    terminal_id: Uuid,
    barrier: &ReconcileTestBarrier,
) {
    let path = harness.paths().runtime_record(terminal_id);
    let before_list: RuntimeRecord = read_json(&path).unwrap();
    let listed = harness
        .client()
        .list_sessions(false)
        .await
        .unwrap()
        .into_iter()
        .find(|view| view.manifest.terminal_id == terminal_id)
        .unwrap();
    assert!(listed.completed_attachment.is_none());
    let after_list: RuntimeRecord = read_json(&path).unwrap();
    assert_eq!(
        after_list.controller_generation,
        before_list.controller_generation
    );
    let host_generation = harness
        .client()
        .status()
        .await
        .unwrap()
        .controller_generation;
    let adopted = harness.client().get_session(terminal_id).await.unwrap();
    assert!(adopted.completed_attachment.is_some());
    let after_get: RuntimeRecord = read_json(&path).unwrap();
    assert_eq!(after_get.controller_generation, host_generation);
    assert!(after_get.controller_generation > before_list.controller_generation);
    let repeated = harness.client().get_session(terminal_id).await.unwrap();
    assert!(repeated.completed_attachment.is_some());
    let after_repeated_get: RuntimeRecord = read_json(&path).unwrap();
    assert_eq!(
        after_repeated_get.controller_generation,
        after_get.controller_generation
    );
    barrier.release().unwrap();
}

pub(super) fn spawn_completed_reconcile_host(home: &Path, barrier_root: &Path) -> SpawnedHost {
    let mut command = Command::new(runtime_binary());
    command
        .args(yolo_launch_contract::ProcessRole::Host.arguments())
        .args([cli::SERVE_COMMAND, cli::HOME_ARGUMENT])
        .arg(home)
        .env(ReconcileTestBarrier::environment_variable(), barrier_root)
        .env(SHELL_ENV, TEST_SHELL)
        .stdin(Stdio::null())
        .stdout(Stdio::null());
    spawn_logged_host(home, &mut command)
}

pub(super) async fn wait_for_completed_reconcile_barrier(barrier: &ReconcileTestBarrier) {
    let deadline = tokio::time::Instant::now() + COMPLETED_RELAY_TIMEOUT;
    while !barrier.reached().unwrap() {
        assert!(tokio::time::Instant::now() < deadline);
        tokio::time::sleep(PROCESS_POLL_INTERVAL).await;
    }
}

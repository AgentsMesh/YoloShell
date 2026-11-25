use crate::worker::tests::*;
const SHUTDOWN_FENCE_EXPECTED_WAITERS: usize = 1;
const SHUTDOWN_FENCE_QUEUE_TIMEOUT: Duration = Duration::from_secs(10);
const SHUTDOWN_FENCE_EARLY_EXIT: &str = "revoked shutdown exited before lifecycle admission";
const SHUTDOWN_FENCE_QUEUE_TIMEOUT_DETAIL: &str =
    "revoked shutdown did not queue for lifecycle admission";

#[tokio::test]
async fn revoked_shutdown_rejects_successor_launch_and_reboot_intent() {
    let successor_fixture = active_projection_fixture();
    let (removed, _) = prepare_revoked_projection(&successor_fixture);
    let runtime = lock_recover(&successor_fixture.state.record).clone();
    atomic_write_json(
        &successor_fixture
            .state
            .paths
            .worker_launch(successor_fixture.terminal_id()),
        &successor_launch(&successor_runtime(&runtime)),
    )
    .unwrap();
    assert_revoked_shutdown_rejected(&successor_fixture, removed).await;

    let intent_fixture = active_projection_fixture();
    let (removed, _) = prepare_revoked_projection(&intent_fixture);
    atomic_write_json(
        &intent_fixture
            .state
            .paths
            .reboot_migration_intent(intent_fixture.terminal_id()),
        &serde_json::json!({}),
    )
    .unwrap();
    assert_revoked_shutdown_rejected(&intent_fixture, removed).await;
}

#[tokio::test]
async fn queued_revoked_shutdown_rejects_successor_launch() {
    let fixture = active_projection_fixture();
    let (removed, _) = prepare_revoked_projection(&fixture);
    let guard = shutdown_fence_guard(&fixture);
    let state = Arc::clone(&fixture.state);
    let shutdown = std::thread::spawn(move || run_revoked_shutdown(state, removed));
    wait_for_shutdown_fence(&fixture, &shutdown);
    let runtime = lock_recover(&fixture.state.record).clone();
    let launch = successor_launch(&successor_runtime(&runtime));
    guard
        .write_json_expected_with_commit_fence(
            &fixture.state.paths.worker_launch(fixture.terminal_id()),
            None,
            &launch,
        )
        .unwrap();
    drop(guard);

    assert!(shutdown.join().unwrap().is_err());
    assert_projection_active(&fixture);
}

#[tokio::test]
async fn queued_revoked_shutdown_rejects_runtime_replacement() {
    let fixture = active_projection_fixture();
    let (removed, _) = prepare_revoked_projection(&fixture);
    let guard = shutdown_fence_guard(&fixture);
    let state = Arc::clone(&fixture.state);
    let shutdown = std::thread::spawn(move || run_revoked_shutdown(state, removed));
    wait_for_shutdown_fence(&fixture, &shutdown);
    let current = lock_recover(&fixture.state.record).clone();
    let path = fixture.state.paths.runtime_record(fixture.terminal_id());
    let (_, identity): (RuntimeRecord, _) =
        guard.namespace().read_json_with_identity(&path).unwrap();
    guard
        .write_json_expected_with_commit_fence(&path, Some(identity), &successor_runtime(&current))
        .unwrap();
    drop(guard);

    assert!(shutdown.join().unwrap().is_err());
    assert_projection_active(&fixture);
}

fn shutdown_fence_guard(
    fixture: &Fixture,
) -> crate::paths::durable_mutation::SessionDurableMutationGuard<'_> {
    crate::paths::durable_mutation::SessionDurableMutationGuard::acquire_participant(
        &fixture.state.paths,
        fixture.state.namespace.lease(),
        fixture.terminal_id(),
    )
    .unwrap()
}

fn run_revoked_shutdown(
    state: Arc<WorkerState>,
    removed: RemovedTerminalManifest,
) -> anyhow::Result<bool> {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(codex_hook::invalidate_projection_for_shutdown(
            &state,
            AgentProjectionShutdown::ManifestRevoked(removed),
        ))
}

fn wait_for_shutdown_fence<T>(fixture: &Fixture, shutdown: &std::thread::JoinHandle<T>) {
    let deadline = Instant::now() + SHUTDOWN_FENCE_QUEUE_TIMEOUT;
    while crate::paths::test_support::queued_session_lifecycle_waiters(
        &fixture.state.paths, fixture.terminal_id(),
    ).unwrap()
        != SHUTDOWN_FENCE_EXPECTED_WAITERS
    {
        assert!(!shutdown.is_finished(), "{SHUTDOWN_FENCE_EARLY_EXIT}");
        assert!(
            Instant::now() < deadline,
            "{SHUTDOWN_FENCE_QUEUE_TIMEOUT_DETAIL}"
        );
        std::thread::yield_now();
    }
}

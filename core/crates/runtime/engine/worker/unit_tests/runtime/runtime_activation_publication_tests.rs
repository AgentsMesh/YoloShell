use crate::worker::tests::*;
const ACTIVATION_PUBLICATION_TIMEOUT: Duration = Duration::from_secs(2);
const ACTIVATION_COLUMN_INCREMENT: u16 = 1;

#[test]
fn runtime_activation_cannot_retire_launch_while_an_agent_holds_lifecycle_authority() {
    let fixture = Fixture::new();
    let (launch, _) = publish_agent_launch_fixture(&fixture);
    let initial = lock_recover(&fixture.state.record).clone();
    let fence = capture_agent_launch_fence(&fixture).unwrap();
    let held = crate::paths::test_support::hold_session_lifecycle_lock(
        &fixture.state.paths,
        fixture.terminal_id(),
    )
    .unwrap();
    let mut expected = initial.clone();
    expected.cols += ACTIVATION_COLUMN_INCREMENT;
    let state = Arc::clone(&fixture.state);
    let next = expected.clone();
    let (blocked, observed) = std::sync::mpsc::channel();
    let publisher = std::thread::spawn(move || {
        crate::paths::test_support::set_private_file_wait_contention_hook(move || {
            blocked.send(()).unwrap();
        });
        publish_runtime_activation(&state.namespace, &state.paths, &next)
    });
    observed
        .recv_timeout(ACTIVATION_PUBLICATION_TIMEOUT)
        .unwrap();
    let launch_still_exact = fence.verify(
        fixture.state.namespace.lease(),
        &fixture.state.paths,
        &initial,
    );
    let retained_launch: Result<WorkerLaunchSpec> = fixture
        .state
        .namespace
        .read_json(&fixture.state.paths.worker_launch(fixture.terminal_id()));
    let retained_runtime: RuntimeRecord = fixture
        .state
        .namespace
        .read_json(&fixture.state.paths.runtime_record(fixture.terminal_id()))
        .unwrap();
    drop(held);
    publisher.join().unwrap().unwrap();
    launch_still_exact.unwrap();
    assert_eq!(retained_launch.unwrap(), launch);
    assert_eq!(retained_runtime, initial);
    assert!(
        fixture
            .state
            .namespace
            .file_is_missing(&fixture.state.paths.worker_launch(fixture.terminal_id()))
            .unwrap()
    );
    let actual: RuntimeRecord = fixture
        .state
        .namespace
        .read_json(&fixture.state.paths.runtime_record(fixture.terminal_id()))
        .unwrap();
    assert_eq!(actual, expected);
}

#[test]
fn runtime_activation_without_launch_still_publishes_through_the_guard() {
    let fixture = Fixture::new();
    let mut record = lock_recover(&fixture.state.record).clone();
    record.cols += ACTIVATION_COLUMN_INCREMENT;
    publish_runtime_activation(&fixture.state.namespace, &fixture.state.paths, &record).unwrap();
    let actual: RuntimeRecord = fixture
        .state
        .namespace
        .read_json(&fixture.state.paths.runtime_record(fixture.terminal_id()))
        .unwrap();
    assert_eq!(actual, record);
}

#[test]
fn runtime_activation_pending_reboot_intent_preserves_exact_launch_and_runtime() {
    let fixture = Fixture::new();
    let (launch, _) = publish_agent_launch_fixture(&fixture);
    let record = lock_recover(&fixture.state.record).clone();
    atomic_write_json(
        &fixture
            .state
            .paths
            .reboot_migration_intent(fixture.terminal_id()),
        &serde_json::json!({}),
    )
    .unwrap();
    let result =
        publish_runtime_activation(&fixture.state.namespace, &fixture.state.paths, &record);
    assert!(result.is_err());
    let retained_launch: WorkerLaunchSpec = fixture
        .state
        .namespace
        .read_json(&fixture.state.paths.worker_launch(fixture.terminal_id()))
        .unwrap();
    let retained_runtime: RuntimeRecord = fixture
        .state
        .namespace
        .read_json(&fixture.state.paths.runtime_record(fixture.terminal_id()))
        .unwrap();
    assert_eq!(retained_launch, launch);
    assert_eq!(retained_runtime, record);
}

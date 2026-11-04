use crate::worker::tests::*;
const EXPECTED_QUEUED_PUBLISHERS: usize = 1;
const PROJECTION_QUEUE_EARLY_EXIT: &str = "projection publisher exited before queueing";
const PROJECTION_QUEUE_TIMEOUT: &str = "projection publisher did not queue before timeout";
const PROJECTION_LOCK_WAIT_TIMEOUT: Duration = Duration::from_secs(10);

#[test]
fn removed_raw_authority_cannot_be_republished_across_resume_boundaries() {
    let fixture = Fixture::new();
    let predecessor = lock_recover(&fixture.state.record).clone();
    let stale = projection_authority(&fixture, &predecessor);
    let store = projection_binding_store(&fixture);
    let authority_store = store.codex_authority_store().unwrap();
    atomic_write_json(authority_store.path(), &serde_json::json!({})).unwrap();
    fixture
        .state
        .namespace
        .remove_file(authority_store.path())
        .unwrap();
    let successor = successor_runtime(&predecessor);
    let launch = successor_launch(&successor);
    atomic_write_json(
        &fixture.state.paths.worker_launch(fixture.terminal_id()),
        &launch,
    )
    .unwrap();

    assert!(publish_as_projection(&fixture, &predecessor, &stale).is_err());
    assert_projection_empty(&fixture);

    fixture
        .state
        .namespace
        .remove_file(&fixture.state.paths.worker_launch(fixture.terminal_id()))
        .unwrap();
    atomic_write_json(
        &fixture.state.paths.runtime_record(fixture.terminal_id()),
        &successor,
    )
    .unwrap();

    assert!(publish_as_projection(&fixture, &predecessor, &stale).is_err());
    assert_projection_empty(&fixture);
}

#[test]
fn queued_predecessor_publisher_cannot_cross_resume_revoke() {
    let fixture = Fixture::new();
    let predecessor = lock_recover(&fixture.state.record).clone();
    let stale = projection_authority(&fixture, &predecessor);
    let store = projection_binding_store(&fixture);
    let authority_store = store.codex_authority_store().unwrap();
    atomic_write_json(authority_store.path(), &serde_json::json!({})).unwrap();
    let expected = fixture
        .state
        .namespace
        .lease()
        .file(authority_store.path())
        .unwrap()
        .inspect(u64::MAX)
        .unwrap()
        .unwrap();
    let guard = crate::paths::durable_mutation::SessionDurableMutationGuard::acquire_participant(
        &fixture.state.paths,
        fixture.state.namespace.lease(),
        fixture.terminal_id(),
    )
    .unwrap();
    let state = Arc::clone(&fixture.state);
    let publisher =
        std::thread::spawn(move || publish_projection_for_state(&state, &predecessor, &stale));
    wait_for_queued_projection_publisher(&fixture, &publisher);
    let successor = successor_runtime(&lock_recover(&fixture.state.record));
    let launch = successor_launch(&successor);
    guard
        .write_json_expected_with_commit_fence(
            &fixture.state.paths.worker_launch(fixture.terminal_id()),
            None,
            &launch,
        )
        .unwrap();
    guard
        .remove_exact_with_replay_boundary(authority_store.path(), expected, u64::MAX, || Ok(()))
        .unwrap();
    drop(guard);

    assert!(publisher.join().unwrap().is_err());
    assert_projection_empty(&fixture);
    fixture
        .state
        .namespace
        .remove_file(&fixture.state.paths.worker_launch(fixture.terminal_id()))
        .unwrap();
    atomic_write_json(
        &fixture.state.paths.runtime_record(fixture.terminal_id()),
        &successor,
    )
    .unwrap();
    let successor_authority = projection_authority(&fixture, &successor);
    publish_as_projection(&fixture, &successor, &successor_authority).unwrap();
    assert_projection_pair(&fixture, &successor_authority);
}

fn wait_for_queued_projection_publisher(
    fixture: &Fixture,
    publisher: &std::thread::JoinHandle<anyhow::Result<()>>,
) {
    let deadline = Instant::now() + PROJECTION_LOCK_WAIT_TIMEOUT;
    while crate::paths::test_support::queued_session_lifecycle_waiters(
        &fixture.state.paths, fixture.terminal_id(),
    ).unwrap()
        != EXPECTED_QUEUED_PUBLISHERS
    {
        assert!(!publisher.is_finished(), "{PROJECTION_QUEUE_EARLY_EXIT}");
        assert!(Instant::now() < deadline, "{PROJECTION_QUEUE_TIMEOUT}");
        std::thread::yield_now();
    }
}

pub(super) fn successor_launch(successor: &RuntimeRecord) -> WorkerLaunchSpec {
    WorkerLaunchSpec {
        schema_version: WORKER_LAUNCH_SCHEMA_VERSION,
        terminal_id: successor.terminal_id,
        runtime_epoch: successor.runtime_epoch,
        worker_id: successor.worker_id,
        socket_path: successor.socket_path.clone(),
        auth_token: successor.auth_token.clone(),
        controller_generation: successor.controller_generation,
        command: successor.command.clone(),
        args: successor.args.clone(),
        cwd: successor.cwd.clone(),
        environment: Default::default(),
        initial_input: None,
        cols: successor.cols,
        rows: successor.rows,
        context: None,
    }
}

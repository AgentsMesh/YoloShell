use crate::worker::tests::*;
const SNAPSHOT_CONCURRENCY_TIMEOUT: Duration = Duration::from_secs(2);
const SNAPSHOT_WAIT_POLL: Duration = Duration::from_millis(2);
const SNAPSHOT_OLD_OUTPUT: &[u8] = b"snapshot-old";
const SNAPSHOT_NEW_OUTPUT: &[u8] = b"snapshot-new";
const SNAPSHOT_PANIC_OUTPUT: &[u8] = b"snapshot-panic";
const SNAPSHOT_RESIZE_HOLDER: &str = "snapshot-resize";
const SNAPSHOT_RESIZE_COLS: u16 = 101;
const SNAPSHOT_RESIZE_ROWS: u16 = 39;
const SNAPSHOT_REPLACEMENT_CONTROLLER_GENERATION: u64 = FIXTURE_CONTROLLER_GENERATION + 1;

pub(super) fn wait_for_snapshot_publication_lock(state: &WorkerState) {
    let deadline = Instant::now() + SNAPSHOT_CONCURRENCY_TIMEOUT;
    loop {
        match state.snapshot_publication.try_lock() {
            Err(std::sync::TryLockError::WouldBlock) => return,
            Err(std::sync::TryLockError::Poisoned(error)) => drop(error.into_inner()),
            Ok(guard) => drop(guard),
        }
        assert!(
            Instant::now() < deadline,
            "snapshot publication did not start"
        );
        std::thread::sleep(SNAPSHOT_WAIT_POLL);
    }
}

#[test]
fn concurrent_snapshot_publication_cannot_overwrite_a_newer_barrier() {
    let fixture = Fixture::new();
    fixture.state.commit_output(SNAPSHOT_OLD_OUTPUT).unwrap();
    fixture
        .state
        .control_test_hooks
        .snapshot_before_publication
        .arm();
    let first_state = Arc::clone(&fixture.state);
    let first = std::thread::spawn(move || first_state.current_snapshot_internal().unwrap());
    fixture
        .state
        .control_test_hooks
        .snapshot_before_publication
        .wait_until_reached();
    fixture.state.commit_output(SNAPSHOT_NEW_OUTPUT).unwrap();
    let second_state = Arc::clone(&fixture.state);
    let second = std::thread::spawn(move || second_state.current_snapshot_internal().unwrap());
    assert!(!second.is_finished());

    fixture
        .state
        .control_test_hooks
        .snapshot_before_publication
        .resume();
    let first = first.join().unwrap();
    let second = second.join().unwrap();
    let durable = load_snapshot(&fixture.state.paths, fixture.terminal_id()).unwrap();
    assert!(second.barrier_sequence > first.barrier_sequence);
    assert_eq!(durable.barrier_sequence, second.barrier_sequence);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn snapshot_waits_for_resize_dimensions_and_terminal_as_one_update() {
    let fixture = Fixture::new();
    let controller = fixture.controller();
    let lease = fixture
        .state
        .acquire_lease(
            controller,
            &fixture.lease_request(SNAPSHOT_RESIZE_HOLDER),
            false,
        )
        .await
        .unwrap();
    let resize = ResizeRequest {
        terminal_id: fixture.terminal_id(),
        runtime_epoch: fixture.state.runtime_epoch(),
        generation: lease.generation,
        writer_capability: lease.writer_capability,
        cols: SNAPSHOT_RESIZE_COLS,
        rows: SNAPSHOT_RESIZE_ROWS,
    };
    fixture
        .state
        .control_test_hooks
        .resize_after_dimensions
        .arm();
    let resize_state = Arc::clone(&fixture.state);
    let resize_task =
        tokio::spawn(async move { resize_state.resize_terminal(controller, &resize).await });
    fixture
        .state
        .control_test_hooks
        .resize_after_dimensions
        .wait_until_reached();
    let snapshot_state = Arc::clone(&fixture.state);
    let snapshot = std::thread::spawn(move || snapshot_state.current_snapshot_internal().unwrap());
    wait_for_snapshot_publication_lock(&fixture.state);
    assert!(!snapshot.is_finished());

    fixture
        .state
        .control_test_hooks
        .resize_after_dimensions
        .resume();
    resize_task.await.unwrap().unwrap();
    let snapshot = snapshot.join().unwrap();
    assert_eq!(
        (snapshot.cols, snapshot.rows),
        (SNAPSHOT_RESIZE_COLS, SNAPSHOT_RESIZE_ROWS)
    );
    assert_eq!(
        lock_recover(&fixture.state.terminal).screen().size(),
        (SNAPSHOT_RESIZE_ROWS, SNAPSHOT_RESIZE_COLS)
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn queued_stale_controller_cannot_publish_snapshot() {
    let fixture = Fixture::new();
    fixture.state.current_snapshot_internal().unwrap();
    let path = fixture.state.paths.snapshot(fixture.terminal_id());
    let before = std::fs::read(&path).unwrap();
    let old_controller = fixture.controller();
    fixture
        .state
        .control_test_hooks
        .controller_before_gate
        .arm();
    let state = Arc::clone(&fixture.state);
    let request = RequestEnvelope::new(
        Uuid::new_v4().to_string(),
        method::GET_SNAPSHOT,
        to_value(&SnapshotRequest {
            terminal_id: fixture.terminal_id(),
            runtime_epoch: fixture.state.runtime_epoch(),
        })
        .unwrap(),
    );
    let stale =
        tokio::spawn(async move { dispatch_request_inner(&request, &state, old_controller).await });
    fixture
        .state
        .control_test_hooks
        .controller_before_gate
        .wait_until_reached()
        .await;
    fixture
        .state
        .adopt_controller(SNAPSHOT_REPLACEMENT_CONTROLLER_GENERATION)
        .await
        .unwrap();
    fixture
        .state
        .control_test_hooks
        .controller_before_gate
        .resume();
    let error = stale.await.unwrap().unwrap_err();
    assert_eq!(error.code, crate::protocol::error_code::CONFLICT);
    assert_eq!(std::fs::read(path).unwrap(), before);
}

#[test]
fn parser_recovery_does_not_invert_snapshot_journal_lock_order() {
    let fixture = Fixture::new();
    fixture.state.control_test_hooks.panic_next_terminal_parse();
    fixture
        .state
        .control_test_hooks
        .parser_recovery_after_dimensions
        .arm();
    let output_state = Arc::clone(&fixture.state);
    let output = std::thread::spawn(move || output_state.commit_output(SNAPSHOT_PANIC_OUTPUT));
    fixture
        .state
        .control_test_hooks
        .parser_recovery_after_dimensions
        .wait_until_reached();
    let snapshot_state = Arc::clone(&fixture.state);
    let snapshot = std::thread::spawn(move || snapshot_state.current_snapshot_internal().unwrap());
    wait_for_snapshot_publication_lock(&fixture.state);
    assert!(matches!(
        fixture.state.dimensions.try_lock(),
        Err(std::sync::TryLockError::WouldBlock)
    ));

    fixture
        .state
        .control_test_hooks
        .parser_recovery_after_dimensions
        .resume();
    output.join().unwrap().unwrap();
    snapshot.join().unwrap();
}

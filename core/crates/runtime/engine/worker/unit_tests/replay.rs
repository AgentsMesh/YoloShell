use crate::worker::tests::*;
const READER_START_TIMEOUT: Duration = Duration::from_secs(1);
const FINALIZATION_WAIT_TIMEOUT: Duration = Duration::from_secs(1);
const FINALIZATION_PENDING_OBSERVATION: Duration = Duration::from_millis(25);
const SHORT_DRAIN_TIMEOUT: Duration = Duration::from_millis(20);
const DRAIN_RETURN_UPPER_BOUND: Duration = Duration::from_millis(500);
const LATE_OUTPUT_OBSERVATION: Duration = Duration::from_millis(50);
const READER_GATE_CHANNEL_CAPACITY: usize = 1;

#[test]
fn replay_snapshot_and_resynchronization_use_durable_sequence_barriers() {
    let fixture = Fixture::new();
    let state = &fixture.state;
    state.journal.lock().unwrap().append(b"one").unwrap();
    state.journal.lock().unwrap().append(b"two").unwrap();
    state.terminal.lock().unwrap().process(b"one");
    state.terminal.lock().unwrap().process(b"two");
    let attached = state.attach_result_internal(0).unwrap();
    assert_eq!(attached.chunks.len(), 2);
    assert_eq!(attached.next_sequence, 3);
    assert!(!attached.resynchronized);
    let resynchronized = state
        .attach_result_internal(FORCE_SNAPSHOT_AFTER_SEQUENCE)
        .unwrap();
    assert!(resynchronized.resynchronized);
    assert_eq!(resynchronized.snapshot.unwrap().barrier_sequence, 2);

    let oversized_chunk = vec![b'x'; 512 * 1024];
    for _ in 0..=(WORKER_MAX_REPLAY_BYTES / oversized_chunk.len()) {
        state
            .journal
            .lock()
            .unwrap()
            .append(&oversized_chunk)
            .unwrap();
    }
    let truncated_replay = state.attach_result_internal(0).unwrap();
    assert!(truncated_replay.resynchronized);
    assert!(truncated_replay.chunks.is_empty());
    assert_eq!(
        truncated_replay.snapshot.unwrap().barrier_sequence,
        3 + (WORKER_MAX_REPLAY_BYTES / oversized_chunk.len()) as u64
    );

    let snapshot = state.current_snapshot_internal().unwrap();
    assert!(
        snapshot
            .data
            .windows(b"onetwo".len())
            .any(|window| window == b"onetwo")
    );
    assert_eq!(
        load_snapshot(&state.paths, fixture.terminal_id())
            .unwrap()
            .cols,
        80
    );
    assert!(load_exit(&state.paths, fixture.terminal_id()).is_none());
}

#[test]
fn compacted_cursor_gap_returns_a_complete_snapshot() {
    let fixture = Fixture::new();
    let state = &fixture.state;
    let bounded = Journal::open_with_max_bytes(
        state.paths.journal(fixture.terminal_id()),
        state.runtime_epoch(),
        140,
    )
    .unwrap();
    *state.journal.lock().unwrap() = bounded;

    assert_eq!(state.commit_output(b"first-record-").unwrap().sequence, 1);
    assert_eq!(state.commit_output(b"second-record-").unwrap().sequence, 2);
    assert_eq!(state.commit_output(b"third-record-").unwrap().sequence, 3);
    assert_eq!(state.journal.lock().unwrap().retained_from_sequence(), 3);
    assert!(state.journal.lock().unwrap().disk_bytes().unwrap() <= 140);

    let stale = state.attach_result_internal(0).unwrap();
    assert!(stale.resynchronized);
    assert!(stale.chunks.is_empty());
    let snapshot = stale.snapshot.unwrap();
    assert_eq!(snapshot.barrier_sequence, 3);
    assert!(
        snapshot
            .data
            .windows(b"first-record-second-record-third-record-".len())
            .any(|window| window == b"first-record-second-record-third-record-")
    );

    let at_snapshot_barrier = state.attach_result_internal(2).unwrap();
    assert!(!at_snapshot_barrier.resynchronized);
    assert_eq!(at_snapshot_barrier.chunks[0].sequence, 3);
}

#[tokio::test]
async fn child_exit_waits_for_reader_commit_and_releases_pty_resources() {
    let fixture = Fixture::new();
    let mut events = fixture.state.events.subscribe();
    let (started_tx, started_rx) = mpsc::sync_channel(READER_GATE_CHANNEL_CAPACITY);
    let (release_tx, release_rx) = mpsc::sync_channel(READER_GATE_CHANNEL_CAPACITY);
    spawn_reader(
        Box::new(DelayedReader {
            started: Some(started_tx),
            release: release_rx,
            exhausted: false,
        }),
        Arc::clone(&fixture.state),
    )
    .unwrap();
    started_rx.recv_timeout(READER_START_TIMEOUT).unwrap();

    let state = Arc::clone(&fixture.state);
    let waiter = std::thread::spawn(move || {
        finalize_child_exit(&state, Some(7), None, FINALIZATION_WAIT_TIMEOUT);
    });
    std::thread::sleep(FINALIZATION_PENDING_OBSERVATION);
    assert!(fixture.state.exit.lock().unwrap().is_none());
    assert_eq!(fixture.state.status().status, TerminalRuntimeStatus::Finalizing);
    assert!(fixture.state.master.lock().unwrap().is_some());
    release_tx.send(b"last-output".to_vec()).unwrap();
    fixture
        .state
        .reader_lifecycle
        .mark_producers_gone_for_test();
    waiter.join().unwrap();

    let exit = fixture.state.exit.lock().unwrap().clone().unwrap();
    assert_eq!(exit.code, Some(7));
    assert_eq!(exit.last_output_sequence, 1);
    assert!(fixture.state.writer.lock().unwrap().is_none());
    assert!(fixture.state.master.lock().unwrap().is_none());
    let lease_error = fixture
        .state
        .get_writer_lease(fixture.controller())
        .await
        .unwrap_err();
    assert_eq!(lease_error.code, lease_error_code::SESSION_NOT_RUNNING);
    assert_eq!(
        fixture
            .state
            .journal
            .lock()
            .unwrap()
            .replay_after(0, 1024)
            .unwrap()[0]
            .data,
        b"last-output"
    );
    assert!(matches!(events.try_recv(), Ok(WorkerEvent::Output(_))));
    assert!(matches!(events.try_recv(), Ok(WorkerEvent::Exited(_))));
}

#[test]
fn reader_drain_timeout_never_overtakes_late_output() {
    let fixture = Fixture::new();
    let (started_tx, started_rx) = mpsc::sync_channel(READER_GATE_CHANNEL_CAPACITY);
    let (release_tx, release_rx) = mpsc::sync_channel(READER_GATE_CHANNEL_CAPACITY);
    spawn_reader(
        Box::new(DelayedReader {
            started: Some(started_tx),
            release: release_rx,
            exhausted: false,
        }),
        Arc::clone(&fixture.state),
    )
    .unwrap();
    started_rx.recv_timeout(READER_START_TIMEOUT).unwrap();

    let started = Instant::now();
    assert!(drain_reader(&fixture.state, SHORT_DRAIN_TIMEOUT));
    assert!(started.elapsed() < DRAIN_RETURN_UPPER_BOUND);
    let state = Arc::clone(&fixture.state);
    let finalizer = std::thread::spawn(move || {
        finalize_child_exit(&state, Some(0), None, SHORT_DRAIN_TIMEOUT);
    });
    std::thread::sleep(LATE_OUTPUT_OBSERVATION);
    assert!(fixture.state.exit.lock().unwrap().is_none());
    release_tx.send(b"late-but-durable".to_vec()).unwrap();
    fixture
        .state
        .reader_lifecycle
        .mark_producers_gone_for_test();
    finalizer.join().unwrap();
    let exit = fixture.state.exit.lock().unwrap().clone().unwrap();
    assert_eq!(exit.last_output_sequence, 1);
    let chunks = fixture
        .state
        .journal
        .lock()
        .unwrap()
        .replay_after(0, usize::MAX)
        .unwrap();
    assert_eq!(chunks[0].data, b"late-but-durable");
}

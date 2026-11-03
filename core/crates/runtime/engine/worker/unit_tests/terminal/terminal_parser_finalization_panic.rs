#[test]
fn parser_recovery_during_finalization_does_not_break_the_attach_stream() {
    let fixture = Fixture::new();
    *lock_recover(&fixture.state.terminal) = vt100::Parser::new_with_callbacks(
        MIN_TERMINAL_ROWS,
        TERMINAL_TEST_UNSAFE_COLUMNS,
        TERMINAL_SCROLLBACK_ROWS,
        yolo_terminal_metadata::MetadataCallbacks::default(),
    );
    *lock_recover(&fixture.state.dimensions) = (MIN_TERMINAL_COLUMNS, MIN_TERMINAL_ROWS);
    fixture.state.child_waited.store(true, Ordering::Release);
    let mut events = fixture.state.events.subscribe();

    commit_reader_output(&fixture.state, TERMINAL_TEST_WIDE_OUTPUT.as_bytes()).unwrap();

    let WorkerEvent::Output(chunk) = events.try_recv().unwrap() else {
        panic!("finalizing parser recovery must not force a resync");
    };
    assert_eq!(chunk.sequence, TERMINAL_TEST_BASELINE_SEQUENCE);
    assert_eq!(
        fixture.state.screen_baseline_sequence(),
        TERMINAL_TEST_BASELINE_SEQUENCE
    );
}

#[test]
fn parser_panic_cannot_replay_or_block_durable_output() {
    let fixture = Fixture::new();
    *lock_recover(&fixture.state.terminal) = vt100::Parser::new_with_callbacks(
        MIN_TERMINAL_ROWS,
        TERMINAL_TEST_UNSAFE_COLUMNS,
        TERMINAL_SCROLLBACK_ROWS,
        yolo_terminal_metadata::MetadataCallbacks::default(),
    );
    *lock_recover(&fixture.state.dimensions) = (MIN_TERMINAL_COLUMNS, MIN_TERMINAL_ROWS);
    let mut events = fixture.state.events.subscribe();

    commit_reader_output(&fixture.state, TERMINAL_TEST_WIDE_OUTPUT.as_bytes()).unwrap();
    let WorkerEvent::ScreenReset(gap) = events.try_recv().unwrap() else {
        panic!("parser recovery must invalidate every attached screen");
    };
    assert_eq!(gap.skipped, SCREEN_RESET_SKIPPED_CHUNKS);
    assert!(gap.resync_required);
    assert!(!fixture.state.reader_lifecycle.has_pending());
    commit_reader_output(&fixture.state, TERMINAL_TEST_RECOVERY_OUTPUT).unwrap();
    commit_reader_output(&fixture.state, TERMINAL_TEST_TAIL_OUTPUT).unwrap();
    assert_eq!(
        fixture.state.screen_baseline_sequence(),
        TERMINAL_TEST_BASELINE_SEQUENCE
    );

    let chunks = fixture
        .state
        .journal
        .lock()
        .unwrap()
        .replay_after(u64::default(), usize::MAX)
        .unwrap();
    assert_eq!(chunks.len(), TERMINAL_TEST_EXPECTED_CHUNKS);
    let mut chunks = chunks.into_iter();
    assert_eq!(
        chunks.next().unwrap().data,
        TERMINAL_TEST_WIDE_OUTPUT.as_bytes()
    );
    assert_eq!(chunks.next().unwrap().data, TERMINAL_TEST_RECOVERY_OUTPUT);
    assert_eq!(chunks.next().unwrap().data, TERMINAL_TEST_TAIL_OUTPUT);
    assert!(chunks.next().is_none());
    let recovered_screen: String = lock_recover(&fixture.state.terminal)
        .screen()
        .contents()
        .chars()
        .filter(|character| !character.is_whitespace())
        .collect();
    assert!(
        recovered_screen.contains("done"),
        "recovered parser screen: {recovered_screen:?}"
    );

    let resynchronized = fixture.state.attach_result_internal(0).unwrap();
    let snapshot = resynchronized.snapshot.as_ref().unwrap();
    assert!(resynchronized.chunks.is_empty());
    assert_eq!(
        snapshot.barrier_sequence,
        TERMINAL_TEST_EXPECTED_CHUNKS as u64
    );
    assert!(
        !snapshot
            .data
            .windows(TERMINAL_TEST_WIDE_OUTPUT.len())
            .any(|window| window == TERMINAL_TEST_WIDE_OUTPUT.as_bytes()),
        "the recovered baseline must not replay the poisoned chunk"
    );
    assert!(!snapshot.data.is_empty());

    let at_baseline = fixture
        .state
        .attach_result_internal(TERMINAL_TEST_BASELINE_SEQUENCE)
        .unwrap();
    assert!(at_baseline.snapshot.is_some());
    assert!(at_baseline.chunks.is_empty());

    let after_baseline = fixture
        .state
        .attach_result_internal(TERMINAL_TEST_AFTER_BASELINE_SEQUENCE)
        .unwrap();
    assert!(after_baseline.snapshot.is_none());
    assert_eq!(after_baseline.chunks.len(), 1);
    assert_eq!(after_baseline.chunks[0].data, TERMINAL_TEST_TAIL_OUTPUT);
}

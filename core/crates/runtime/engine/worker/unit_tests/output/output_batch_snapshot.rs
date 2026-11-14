use super::*;

const BASE_OUTPUT: &[u8] = b".";
const FIRST_OUTPUT: &[u8] = b"batch-first";
const FINAL_OUTPUT: &[u8] = b"batch-final";
const BATCH_CHUNKS: u64 = 2;

fn stage_batch(state: &WorkerState) {
    stage_reader_output(state, FIRST_OUTPUT).unwrap();
    state
        .reader_lifecycle
        .begin_output_read()
        .unwrap()
        .stage_without_attempt(FINAL_OUTPUT)
        .unwrap();
}

fn contains_final_output(snapshot: &TerminalSnapshot) -> bool {
    snapshot
        .data
        .windows(FINAL_OUTPUT.len())
        .any(|window| window == FINAL_OUTPUT)
}

#[test]
fn checkpoint_crossed_inside_batch_contains_the_entire_committed_batch() {
    let fixture = Fixture::new();
    for _ in 1..PTY_SNAPSHOT_INTERVAL_CHUNKS {
        fixture.state.commit_output(BASE_OUTPUT).unwrap();
    }
    stage_batch(&fixture.state);

    assert_eq!(
        deliver_pending_with_retry(&fixture.state).unwrap(),
        PendingDelivery::Delivered
    );

    let snapshot = load_snapshot(&fixture.state.paths, fixture.terminal_id()).unwrap();
    let journal = lock_recover(&fixture.state.journal);
    assert_eq!(snapshot.barrier_sequence, journal.last_sequence());
    assert!(snapshot.barrier_sequence > PTY_SNAPSHOT_INTERVAL_CHUNKS);
    assert!(contains_final_output(&snapshot));
}

#[test]
fn concurrent_snapshot_waits_for_entire_batch_projection_after_parser_recovery() {
    let fixture = Fixture::new();
    stage_batch(&fixture.state);
    let hooks = &fixture.state.control_test_hooks;
    hooks.panic_next_terminal_parse();
    hooks.parser_recovery_after_dimensions.arm();
    hooks.snapshot_before_publication.arm();
    let output_state = Arc::clone(&fixture.state);
    let output = std::thread::spawn(move || deliver_pending_with_retry(&output_state));
    hooks.parser_recovery_after_dimensions.wait_until_reached();

    let journal_held_during_projection = matches!(
        fixture.state.journal.try_lock(),
        Err(std::sync::TryLockError::WouldBlock)
    );
    let snapshot_state = Arc::clone(&fixture.state);
    let snapshot = std::thread::spawn(move || snapshot_state.current_snapshot_internal());
    wait_for_snapshot_publication_lock(&fixture.state);

    hooks.parser_recovery_after_dimensions.resume();
    hooks.snapshot_before_publication.wait_until_reached();
    hooks.snapshot_before_publication.resume();
    assert_eq!(output.join().unwrap().unwrap(), PendingDelivery::Delivered);
    let snapshot = snapshot.join().unwrap().unwrap();

    assert!(journal_held_during_projection);
    assert_eq!(snapshot.barrier_sequence, BATCH_CHUNKS);
    assert!(contains_final_output(&snapshot));
}

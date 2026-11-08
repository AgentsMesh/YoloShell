use crate::worker::tests::*;
const EXHAUSTED_SEQUENCE: u64 = u64::MAX;

#[test]
fn attachment_producer_force_cursor_returns_a_snapshot() {
    const INITIAL_SEQUENCE: u64 = 0;

    let fixture = Fixture::new();

    let attached = fixture
        .state
        .attach_result_internal(FORCE_SNAPSHOT_AFTER_SEQUENCE)
        .unwrap();

    assert!(attached.resynchronized);
    assert!(attached.chunks.is_empty());
    assert_eq!(
        attached.snapshot.unwrap().barrier_sequence,
        INITIAL_SEQUENCE
    );
}

#[test]
fn attachment_producer_rejects_an_ahead_cursor() {
    let fixture = Fixture::new();
    let error = fixture
        .state
        .attach_result_internal(ATTACHMENT_SEQUENCE_INCREMENT)
        .unwrap_err();

    assert_eq!(error.to_string(), ATTACHMENT_CURSOR_AHEAD_CONTEXT);
}

#[test]
fn attachment_producer_rejects_an_exhausted_snapshot_barrier() {
    let fixture = Fixture::new();
    let runtime_epoch = fixture.state.runtime_epoch();
    let error = fixture
        .state
        .attach_result(FORCE_SNAPSHOT_AFTER_SEQUENCE, || {
            Ok(TerminalSnapshot {
                runtime_epoch,
                barrier_sequence: EXHAUSTED_SEQUENCE,
                cols: FIXTURE_TERMINAL_COLUMNS,
                rows: FIXTURE_TERMINAL_ROWS,
                data: Vec::new(),
            })
        })
        .unwrap_err();

    assert_eq!(error.to_string(), ATTACHMENT_CURSOR_EXHAUSTED_CONTEXT);
}

#[test]
fn attachment_producer_rejects_an_exhausted_journal_sequence() {
    let fixture = Fixture::new();
    fixture
        .state
        .journal
        .lock()
        .unwrap()
        .set_last_sequence_for_test(EXHAUSTED_SEQUENCE);
    let error = fixture
        .state
        .attach_result_internal(EXHAUSTED_SEQUENCE - ATTACHMENT_SEQUENCE_INCREMENT)
        .unwrap_err();

    assert_eq!(error.to_string(), ATTACHMENT_CURSOR_EXHAUSTED_CONTEXT);
}

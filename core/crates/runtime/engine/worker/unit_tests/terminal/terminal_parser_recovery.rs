use crate::worker::tests::*;
use crate::model::{MIN_TERMINAL_COLUMNS, MIN_TERMINAL_ROWS};

const TERMINAL_TEST_UNSAFE_COLUMNS: u16 = 1;
const TERMINAL_TEST_UNSAFE_ROWS: u16 = 1;
const TERMINAL_TEST_WIDE_OUTPUT: &str = "\u{754c}";
const TERMINAL_TEST_RECOVERY_OUTPUT: &[u8] = b"ok";
const TERMINAL_TEST_TAIL_OUTPUT: &[u8] = b"done";
const TERMINAL_TEST_HOLDER: &str = "terminal-size";
const TERMINAL_TEST_EXPECTED_CHUNKS: usize = 3;
const TERMINAL_TEST_BASELINE_SEQUENCE: u64 = 1;
const TERMINAL_TEST_AFTER_BASELINE_SEQUENCE: u64 = 2;
const TERMINAL_TEST_RESIZE_INCREMENT: u16 = 1;
const TERMINAL_TEST_RUNTIME_LINK: &str = "runtime-publication-link";

#[tokio::test]
async fn rejected_resize_publication_preserves_runtime_authority_and_can_retry() {
    let fixture = Fixture::new();
    let controller = fixture.controller();
    let lease = fixture
        .state
        .acquire_lease(
            controller,
            &fixture.lease_request(TERMINAL_TEST_HOLDER),
            false,
        )
        .await
        .unwrap();
    let path = fixture.state.paths.runtime_record(fixture.terminal_id());
    let before: (RuntimeRecord, _) = fixture
        .state
        .namespace
        .read_json_with_identity(&path)
        .unwrap();
    let request = ResizeRequest {
        terminal_id: fixture.terminal_id(),
        runtime_epoch: fixture.state.runtime_epoch(),
        generation: lease.generation,
        writer_capability: lease.writer_capability,
        cols: before.0.cols + TERMINAL_TEST_RESIZE_INCREMENT,
        rows: before.0.rows,
    };
    let linked = fixture._filesystem.scratch_path(TERMINAL_TEST_RUNTIME_LINK);
    std::fs::hard_link(&path, &linked).unwrap();
    assert!(
        fixture
            .state
            .resize_terminal(controller, &request)
            .await
            .is_err()
    );
    assert_eq!(*lock_recover(&fixture.state.record), before.0);
    assert_eq!(
        *lock_recover(&fixture.state.dimensions),
        (before.0.cols, before.0.rows),
        "a rejected publication must restore the live geometry"
    );
    let terminal = lock_recover(&fixture.state.terminal);
    let (rows, cols) = terminal.screen().size();
    drop(terminal);
    assert_eq!(
        (cols, rows),
        (before.0.cols, before.0.rows),
        "the screen parser must match the restored geometry"
    );
    std::fs::remove_file(linked).unwrap();
    let rejected: (RuntimeRecord, _) = fixture
        .state
        .namespace
        .read_json_with_identity(&path)
        .unwrap();
    assert_eq!(rejected.0, before.0);
    assert!(rejected.1.same_content_version(before.1));
    fixture
        .state
        .resize_terminal(controller, &request)
        .await
        .unwrap();
    let persisted: RuntimeRecord = fixture.state.namespace.read_json(&path).unwrap();
    assert_eq!(*lock_recover(&fixture.state.record), persisted);
    assert_eq!(persisted.cols, request.cols);
    assert_eq!(persisted.rows, request.rows);
}

#[tokio::test]
async fn same_size_resize_preserves_runtime_file_identity() {
    let fixture = Fixture::new();
    let lease = fixture
        .state
        .acquire_lease(
            fixture.controller(),
            &fixture.lease_request(TERMINAL_TEST_HOLDER),
            false,
        )
        .await
        .unwrap();
    let runtime_path = fixture.state.paths.runtime_record(fixture.terminal_id());
    let before: (RuntimeRecord, _) = fixture
        .state
        .namespace
        .read_json_with_identity(&runtime_path)
        .unwrap();

    fixture
        .state
        .resize_terminal(
            fixture.controller(),
            &ResizeRequest {
                terminal_id: fixture.terminal_id(),
                runtime_epoch: fixture.state.runtime_epoch(),
                generation: lease.generation,
                writer_capability: lease.writer_capability,
                cols: before.0.cols,
                rows: before.0.rows,
            },
        )
        .await
        .unwrap();

    let after: (RuntimeRecord, _) = fixture
        .state
        .namespace
        .read_json_with_identity(&runtime_path)
        .unwrap();
    assert_eq!(after, before);
}

#[tokio::test]
async fn one_column_resize_is_normalized_before_wide_output() {
    let fixture = Fixture::new();
    let lease = fixture
        .state
        .acquire_lease(
            fixture.controller(),
            &fixture.lease_request(TERMINAL_TEST_HOLDER),
            false,
        )
        .await
        .unwrap();
    fixture
        .state
        .resize_terminal(
            fixture.controller(),
            &ResizeRequest {
                terminal_id: fixture.terminal_id(),
                runtime_epoch: fixture.state.runtime_epoch(),
                generation: lease.generation,
                writer_capability: lease.writer_capability,
                cols: TERMINAL_TEST_UNSAFE_COLUMNS,
                rows: TERMINAL_TEST_UNSAFE_ROWS,
            },
        )
        .await
        .unwrap();

    assert_eq!(
        *lock_recover(&fixture.state.dimensions),
        (MIN_TERMINAL_COLUMNS, MIN_TERMINAL_ROWS)
    );
    commit_reader_output(&fixture.state, TERMINAL_TEST_WIDE_OUTPUT.as_bytes()).unwrap();
    let snapshot = fixture.state.current_snapshot_internal().unwrap();
    assert_eq!(snapshot.cols, MIN_TERMINAL_COLUMNS);
    assert_eq!(snapshot.rows, MIN_TERMINAL_ROWS);
    assert!(
        snapshot
            .data
            .windows(TERMINAL_TEST_WIDE_OUTPUT.len())
            .any(|window| window == TERMINAL_TEST_WIDE_OUTPUT.as_bytes())
    );
}


include!("terminal_parser_finalization_panic.rs");

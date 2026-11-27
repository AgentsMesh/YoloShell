use crate::worker::tests::*;
const PRESENTATION_TITLE_START: &[u8] = b"\x1b]2;Codex;";
const PRESENTATION_TITLE_END: &[u8] = b"working\x1b\\";
const PRESENTATION_TITLE: &str = "Codex;working";
const PRESENTATION_NOTIFY: &[u8] = b"\x1b]777;notify;Codex;Review ready\x07";
const PRESENTATION_BODY: &str = "Review ready";
const PRESENTATION_PANIC_OUTPUT: &[u8] = b"parser recovery";
const PRESENTATION_FIRST_EVENT: u64 = 1;

#[test]
fn terminal_presentation_status_and_snapshot_share_committed_output() {
    let fixture = Fixture::new();
    fixture
        .state
        .commit_output(PRESENTATION_TITLE_START)
        .unwrap();
    assert!(fixture.state.status().presentation.title.is_none());
    fixture.state.commit_output(PRESENTATION_TITLE_END).unwrap();
    fixture.state.commit_output(PRESENTATION_NOTIFY).unwrap();
    let status = fixture.state.status();
    assert_eq!(
        status.presentation.title.as_deref(),
        Some(PRESENTATION_TITLE)
    );
    assert_eq!(
        status.presentation.notification_sequence,
        PRESENTATION_FIRST_EVENT
    );
    assert_eq!(
        status.presentation.notification.unwrap().body,
        PRESENTATION_BODY
    );
    let snapshot = fixture.state.current_snapshot_internal().unwrap();
    assert_eq!(snapshot.barrier_sequence, status.last_output_sequence);
    let mut restored = new_terminal_parser(snapshot.cols, snapshot.rows);
    restored.process(&snapshot.data);
    assert_eq!(
        restored.callbacks().presentation().title.as_deref(),
        Some(PRESENTATION_TITLE)
    );
    assert_eq!(
        restored.callbacks().presentation().notification_sequence,
        u64::default()
    );
    let title = load_presentation(
        &fixture.state.paths,
        fixture.terminal_id(),
        snapshot.runtime_epoch,
    );
    assert_eq!(title.title.as_deref(), Some(PRESENTATION_TITLE));
    assert!(title.notification.is_none());
    assert_eq!(
        load_presentation(&fixture.state.paths, fixture.terminal_id(), Uuid::new_v4()),
        crate::model::TerminalPresentation::default(),
    );
}

#[test]
fn terminal_presentation_survives_screen_recovery_without_repeating_notification() {
    let fixture = Fixture::new();
    fixture
        .state
        .commit_output(PRESENTATION_TITLE_START)
        .unwrap();
    fixture.state.commit_output(PRESENTATION_TITLE_END).unwrap();
    fixture.state.commit_output(PRESENTATION_NOTIFY).unwrap();
    let before = fixture.state.status().presentation;
    fixture.state.control_test_hooks.panic_next_terminal_parse();
    fixture
        .state
        .commit_output(PRESENTATION_PANIC_OUTPUT)
        .unwrap();
    assert_eq!(fixture.state.status().presentation, before);
    assert_eq!(
        Fixture::new().state.status().presentation,
        Default::default()
    );
}

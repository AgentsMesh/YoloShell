use crate::fixtures::*;

#[test]
fn cursor_report_uses_origin_relative_coordinates_and_saved_origin_mode() {
    for query in [ORIGIN_QUERY, SAVED_ORIGIN_QUERY] {
        let mut fixture = Fixture::new();
        assert_eq!(fixture.live(query), [THIRD_REPORT.to_vec()]);
    }
}

#[test]
fn wrap_pending_reports_the_last_terminal_column_without_changing_cursor_state() {
    let mut fixture = Fixture::new();
    fixture.quiet(FILL_ROW);
    let cursor = fixture.parser.screen().cursor_position();
    assert_eq!(fixture.live(CPR), [LAST_COLUMN_REPORT.to_vec()]);
    assert_eq!(fixture.parser.screen().cursor_position(), cursor);
}

#[test]
fn scrollback_display_does_not_change_the_pty_cursor_reply() {
    let mut fixture = Fixture::new();
    fixture.quiet(SCROLLBACK_QUERY);
    fixture.parser.screen_mut().set_scrollback(SCROLLBACK);
    assert_eq!(fixture.live(CPR), [THIRD_REPORT.to_vec()]);
    assert!(fixture.parser.screen().scrollback() > usize::default());
}

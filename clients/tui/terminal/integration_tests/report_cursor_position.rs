const ROWS: u16 = 8;
const COLS: u16 = 12;
const SCROLLBACK: usize = 32;
const ORIGIN_REGION: &[u8] = b"\x1b[3;6r\x1b[?6h\x1b[2;4H";
const SCREEN_POSITION: &[u8] = b"\x1b[?6l\x1b[5;7H";
const WRAP_ROWS: u16 = 3;
const WRAP_COLS: u16 = 4;

fn parser() -> vt100::Parser {
    vt100::Parser::new(ROWS, COLS, SCROLLBACK)
}

#[test]
fn report_is_one_based_and_existing_raw_cursor_api_stays_unchanged() {
    let mut terminal = parser();
    assert_eq!(terminal.screen().cursor_position(), (0, 0));
    assert_eq!(terminal.screen().report_cursor_position(), (1, 1));
    terminal.process(SCREEN_POSITION);
    assert_eq!(terminal.screen().cursor_position(), (4, 6));
    assert_eq!(terminal.screen().report_cursor_position(), (5, 7));
}

#[test]
fn origin_mode_reports_from_scroll_top_and_reset_reports_from_screen() {
    let mut terminal = parser();
    terminal.process(ORIGIN_REGION);
    assert_eq!(terminal.screen().cursor_position(), (3, 3));
    assert_eq!(terminal.screen().report_cursor_position(), (2, 4));
    terminal.process(b"\x1b[99;99H");
    assert_eq!(terminal.screen().report_cursor_position(), (4, COLS));
    terminal.process(SCREEN_POSITION);
    assert_eq!(terminal.screen().report_cursor_position(), (5, 7));
}

#[test]
fn wrap_pending_reports_last_column_without_consuming_wrap_or_mutating_screen() {
    let mut terminal = vt100::Parser::new(WRAP_ROWS, WRAP_COLS, SCROLLBACK);
    terminal.process(b"abcd");
    let raw = terminal.screen().cursor_position();
    let contents = terminal.screen().contents();
    assert_eq!(raw, (0, WRAP_COLS));
    assert_eq!(terminal.screen().report_cursor_position(), (1, WRAP_COLS));
    assert_eq!(terminal.screen().report_cursor_position(), (1, WRAP_COLS));
    assert_eq!(terminal.screen().cursor_position(), raw);
    assert_eq!(terminal.screen().contents(), contents);
    terminal.process(b"e");
    assert_eq!(terminal.screen().report_cursor_position(), (2, 2));
}

#[test]
fn saved_origin_mode_and_position_are_used_after_restore() {
    let mut terminal = parser();
    terminal.process(ORIGIN_REGION);
    terminal.process(b"\x1b7");
    terminal.process(SCREEN_POSITION);
    terminal.process(b"\x1b8");
    assert_eq!(terminal.screen().cursor_position(), (3, 3));
    assert_eq!(terminal.screen().report_cursor_position(), (2, 4));
}

#[test]
fn alternate_grid_uses_its_own_origin_and_restores_the_primary_grid() {
    let mut terminal = parser();
    terminal.process(ORIGIN_REGION);
    terminal.process(b"\x1b[?1049h\x1b[?6l\x1b[7;9H");
    assert_eq!(terminal.screen().report_cursor_position(), (7, 9));
    terminal.process(b"\x1b[4;7r\x1b[?6h\x1b[3;2H");
    assert_eq!(terminal.screen().cursor_position(), (5, 1));
    assert_eq!(terminal.screen().report_cursor_position(), (3, 2));
    terminal.process(b"\x1b[?1049l");
    assert_eq!(terminal.screen().report_cursor_position(), (2, 4));
}

#[test]
fn display_scrollback_never_changes_live_report_coordinates() {
    let mut terminal = parser();
    terminal.process(b"1\r\n2\r\n3\r\n4\r\n5\r\n6\r\n7\r\n8\r\n9\r\n");
    terminal.process(ORIGIN_REGION);
    let report = terminal.screen().report_cursor_position();
    terminal.screen_mut().set_scrollback(SCROLLBACK);
    assert!(terminal.screen().scrollback() > 0);
    assert_eq!(terminal.screen().report_cursor_position(), report);
}

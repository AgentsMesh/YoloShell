use crate::fixtures::*;

#[test]
fn displayed_scrollback_is_frozen_without_retaining_the_original_history() {
    let mut parser = parser(SCROLLBACK_TEXT);
    parser.screen_mut().set_scrollback(usize::from(THIRD));
    let selection = selected(&parser, point(FIRST, FIRST), point(NEXT, THIRD));
    assert_eq!(selection.text().unwrap(), "one\ntwo");
    assert_eq!(selection.screen().scrollback(), usize::default());
    let mut restored = vt100::Parser::new(ROWS, COLUMNS, SCROLLBACK);
    restored.process(&selection.screen().contents_formatted());
    restored.screen_mut().set_scrollback(SCROLLBACK);
    assert_eq!(restored.screen().scrollback(), usize::default());
    parser.screen_mut().set_scrollback(usize::default());
    parser.process(REPLACEMENT);
    assert_eq!(selection.text().unwrap(), "one\ntwo");
    assert_eq!(
        selection.screen().rows(FIRST, COLUMNS).next().unwrap(),
        "one"
    );
}

#[test]
fn live_provider_output_does_not_change_an_in_progress_drag_or_its_frozen_display() {
    let mut parser = parser(UNICODE_TEXT.as_bytes());
    let mut selection =
        Selection::begin(parser.screen(), ROWS, COLUMNS, point(FIRST, THIRD)).unwrap();
    parser.process(REPLACEMENT);
    selection.extend(point(FIRST, FOURTH)).unwrap();
    selection.finish(point(FIRST, FOURTH)).unwrap();
    assert_eq!(selection.text().unwrap(), "界e\u{301}");
    assert_eq!(selection.screen().contents(), UNICODE_TEXT);
    assert_ne!(selection.screen().contents(), parser.screen().contents());
}

#[test]
fn frozen_display_preserves_styled_visible_cells_across_physical_soft_wraps() {
    let parser = parser(b"\x1b[31mabcdefgh\x1b[1;44mijklmnopq");
    let selection = selected(&parser, point(FIRST, FIRST), point(THIRD, FIRST));
    assert_eq!(selection.text().unwrap(), "abcdefghijklmnopq");
    for row in FIRST..ROWS {
        for column in FIRST..COLUMNS {
            assert_eq!(
                selection.screen().cell(row, column),
                parser.screen().cell(row, column)
            );
        }
    }
}

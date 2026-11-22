use crate::fixtures::*;

#[test]
fn reverse_and_forward_drags_select_the_same_inclusive_cells() {
    let parser = parser(FIRST_TEXT);
    let start = point(FIRST, THIRD);
    let end = point(NEXT, FOURTH);
    for (anchor, head) in [(start, end), (end, start)] {
        let selection = selected(&parser, anchor, head);
        assert_eq!(selection.range(), (start, end));
        assert_eq!(selection.text().unwrap(), "cdefgh\nijkl");
        assert!(selection.contains(point(FIRST, LAST_COLUMN)));
        assert!(selection.contains(point(NEXT, FIRST)));
        assert!(!selection.contains(point(FIRST, NEXT)));
        assert!(!selection.contains(point(THIRD, FIRST)));
        assert!(!selection.contains(point(FIRST, COLUMNS)));
    }
}

#[test]
fn a_drag_cannot_copy_or_move_outside_the_visible_snapshot() {
    let parser = parser(FIRST_TEXT);
    let mut selection =
        Selection::begin(parser.screen(), ROWS, COLUMNS, point(FIRST, FIRST)).unwrap();
    assert!(selection.is_dragging());
    assert_eq!(selection.dimensions(), (ROWS, COLUMNS));
    assert_eq!(selection.text(), Err(SelectionError::Incomplete));
    assert_eq!(selection.clipboard(), Err(SelectionError::Incomplete));
    assert_eq!(
        selection.extend(point(ROWS, FIRST)),
        Err(SelectionError::InvalidPoint)
    );
    assert_eq!(
        selection.finish(point(FIRST, COLUMNS)),
        Err(SelectionError::InvalidPoint)
    );
    selection.extend(point(FIRST, NEXT)).unwrap();
    selection.finish(point(FIRST, THIRD)).unwrap();
    assert!(!selection.is_dragging());
    assert_eq!(selection.text().unwrap(), "abc");
    assert_eq!(
        selection.extend(point(FIRST, FOURTH)),
        Err(SelectionError::InvalidPoint)
    );
}

#[test]
fn selection_tracks_screen_replacement_without_changing_its_frozen_text() {
    let mut parser = parser(COLORED_TEXT);
    let selection = selected(&parser, point(FIRST, FIRST), point(FIRST, THIRD));
    assert!(selection.matches(parser.screen()));
    assert!(selection.screen().hide_cursor());
    for row in FIRST..ROWS {
        for column in FIRST..COLUMNS {
            assert_eq!(
                selection.screen().cell(row, column),
                parser.screen().cell(row, column)
            );
        }
    }
    parser.process(REPLACEMENT);
    assert!(!selection.matches(parser.screen()));
    assert_eq!(selection.text().unwrap(), "red");
    assert_eq!(
        selection.screen().rows(FIRST, COLUMNS).next().unwrap(),
        "red"
    );
}

#[test]
fn a_different_screen_size_does_not_match_the_captured_source() {
    let parser = parser(FIRST_TEXT);
    let selection = selected(&parser, point(FIRST, FIRST), point(FIRST, NEXT));
    let changed = vt100::Parser::new(ROWS + NEXT, COLUMNS, SCROLLBACK);
    assert!(!selection.matches(changed.screen()));
}

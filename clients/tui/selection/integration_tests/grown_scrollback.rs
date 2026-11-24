use yolo_tui_selection::{CellPoint, Selection, SelectionError};

const ROWS: u16 = 8;
const OLD_COLUMNS: u16 = 4;
const NEW_COLUMNS: u16 = 20;
const HISTORY_ROWS: usize = 16;
const FIRST: u16 = 0;
const STEP: u16 = 1;
const HISTORY_LINE: &[u8] = b"old\r\n";
const EXPECTED_TEXT: &str = "old";
const EXPECTED_COPY: &[u8] = b"\x1b]52;c;b2xk\x1b\\";

#[test]
fn growing_the_grid_keeps_narrow_scrollback_padding_selectable_and_frozen() {
    let mut parser = vt100::Parser::new(ROWS, OLD_COLUMNS, HISTORY_ROWS);
    parser.process(&HISTORY_LINE.repeat(usize::from(ROWS + STEP)));
    parser.screen_mut().set_size(ROWS, NEW_COLUMNS);
    parser.screen_mut().set_scrollback(usize::from(STEP));
    assert!(parser.screen().cell(FIRST, OLD_COLUMNS).is_none());
    let mut selection = Selection::begin(
        parser.screen(),
        ROWS,
        NEW_COLUMNS,
        CellPoint {
            row: FIRST,
            column: FIRST,
        },
    )
    .unwrap();
    selection
        .finish(CellPoint {
            row: FIRST,
            column: NEW_COLUMNS - STEP,
        })
        .unwrap();
    assert!(selection.matches(parser.screen()));
    assert_eq!(selection.text().unwrap(), EXPECTED_TEXT);
    assert_eq!(selection.clipboard().unwrap().as_bytes(), EXPECTED_COPY);
    assert!(
        selection
            .screen()
            .cell(FIRST, OLD_COLUMNS)
            .is_some_and(|cell| !cell.has_contents())
    );
    assert_eq!(
        selection.screen().rows(FIRST, NEW_COLUMNS).next().unwrap(),
        EXPECTED_TEXT
    );
    assert_eq!(selection.screen().scrollback(), usize::default());
    let mut padding = Selection::begin(
        parser.screen(),
        ROWS,
        NEW_COLUMNS,
        CellPoint {
            row: FIRST,
            column: OLD_COLUMNS,
        },
    )
    .unwrap();
    padding
        .finish(CellPoint {
            row: FIRST,
            column: NEW_COLUMNS - STEP,
        })
        .unwrap();
    assert_eq!(padding.text(), Err(SelectionError::EmptySelection));
}

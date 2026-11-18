use crate::fixtures::*;
use yolo_tui_selection::{MAX_SELECTION_BYTES, MAX_SELECTION_CELLS};

#[test]
fn invalid_viewports_and_points_fail_before_a_selection_is_created() {
    let parser = parser(FIRST_TEXT);
    for (rows, columns, anchor, expected) in [
        (
            FIRST,
            COLUMNS,
            point(FIRST, FIRST),
            SelectionError::EmptyViewport,
        ),
        (
            ROWS,
            FIRST,
            point(FIRST, FIRST),
            SelectionError::EmptyViewport,
        ),
        (
            ROWS + NEXT,
            COLUMNS,
            point(FIRST, FIRST),
            SelectionError::InvalidPoint,
        ),
        (
            ROWS,
            COLUMNS + NEXT,
            point(FIRST, FIRST),
            SelectionError::InvalidPoint,
        ),
        (
            ROWS,
            COLUMNS,
            point(ROWS, FIRST),
            SelectionError::InvalidPoint,
        ),
    ] {
        assert_eq!(
            Selection::begin(parser.screen(), rows, columns, anchor).unwrap_err(),
            expected
        );
    }
}

#[test]
fn both_visible_cells_and_the_frozen_source_grid_are_bounded() {
    let parser = vt100::Parser::new(OVERSIZE_AXIS, OVERSIZE_AXIS, SCROLLBACK);
    for columns in [OVERSIZE_AXIS, NEXT] {
        assert_eq!(
            Selection::begin(parser.screen(), OVERSIZE_AXIS, columns, point(FIRST, FIRST))
                .unwrap_err(),
            SelectionError::CaptureLimit
        );
    }
    let parser = vt100::Parser::new(CAPTURE_AXIS, CAPTURE_AXIS, SCROLLBACK);
    assert_eq!(usize::from(CAPTURE_AXIS).pow(2), MAX_SELECTION_CELLS);
    assert!(
        Selection::begin(
            parser.screen(),
            CAPTURE_AXIS,
            CAPTURE_AXIS,
            point(FIRST, FIRST)
        )
        .is_ok()
    );
}

#[test]
fn copy_payload_limit_accepts_exact_budget_and_rejects_the_whole_larger_selection() {
    let mut parser = vt100::Parser::new(COPY_ROWS, COPY_COLUMNS, SCROLLBACK);
    parser.process(&vec![FILL_BYTE; MAX_SELECTION_BYTES + usize::from(NEXT)]);
    let exact_end = MAX_SELECTION_BYTES - usize::from(NEXT);
    let larger_end = MAX_SELECTION_BYTES;
    for (last, expected) in [
        (exact_end, Ok(MAX_SELECTION_BYTES)),
        (larger_end, Err(SelectionError::CopyLimit)),
    ] {
        let end = point(
            (last / usize::from(COPY_COLUMNS)) as u16,
            (last % usize::from(COPY_COLUMNS)) as u16,
        );
        let selection = selected(&parser, point(FIRST, FIRST), end);
        assert_eq!(selection.text().map(|text| text.len()), expected);
    }
}

#[test]
fn selection_errors_have_actionable_messages() {
    for error in [
        SelectionError::EmptyViewport,
        SelectionError::CaptureLimit,
        SelectionError::InvalidPoint,
        SelectionError::CopyLimit,
        SelectionError::Incomplete,
        SelectionError::EmptySelection,
    ] {
        assert!(!error.to_string().is_empty());
    }
}

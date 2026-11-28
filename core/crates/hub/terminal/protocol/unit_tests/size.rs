use crate::fixtures::roundtrip;
use yolo_hub_terminal_protocol::*;

#[test]
fn sizes_roundtrip_at_every_boundary() {
    for cols in [
        MIN_TERMINAL_COLUMNS,
        DEFAULT_TERMINAL_COLUMNS,
        MAX_TERMINAL_COLUMNS,
    ] {
        for rows in [MIN_TERMINAL_ROWS, DEFAULT_TERMINAL_ROWS, MAX_TERMINAL_ROWS] {
            let size = TerminalSize::new(cols, rows).unwrap();
            assert_eq!((size.cols(), size.rows()), (cols, rows));
            roundtrip(&size);
        }
    }
    let size = TerminalSize::default();
    assert_eq!(
        (size.cols(), size.rows()),
        (DEFAULT_TERMINAL_COLUMNS, DEFAULT_TERMINAL_ROWS)
    );
}

#[test]
fn reject_out_of_range_dimensions() {
    for cols in [
        0,
        MIN_TERMINAL_COLUMNS - 1,
        MAX_TERMINAL_COLUMNS + 1,
        u16::MAX,
    ] {
        assert_eq!(
            TerminalSize::new(cols, DEFAULT_TERMINAL_ROWS),
            Err(PublicError::InvalidSize)
        );
    }
    for rows in [0, MIN_TERMINAL_ROWS - 1, MAX_TERMINAL_ROWS + 1, u16::MAX] {
        assert_eq!(
            TerminalSize::new(DEFAULT_TERMINAL_COLUMNS, rows),
            Err(PublicError::InvalidSize)
        );
    }
}

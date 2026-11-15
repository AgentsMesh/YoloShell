#[path = "constants.rs"]
mod constants;
pub use constants::*;
pub use yolo_tui_selection::{CellPoint, Selection, SelectionError};

pub fn parser(bytes: &[u8]) -> vt100::Parser {
    let mut parser = vt100::Parser::new(ROWS, COLUMNS, SCROLLBACK);
    parser.process(bytes);
    parser
}

pub fn point(row: u16, column: u16) -> CellPoint {
    CellPoint { row, column }
}

pub fn selected(parser: &vt100::Parser, start: CellPoint, end: CellPoint) -> Selection {
    let (rows, columns) = parser.screen().size();
    let mut selection = Selection::begin(parser.screen(), rows, columns, start).unwrap();
    selection.finish(end).unwrap();
    selection
}

use crate::{SelectionError, constants::*};

/// Rebuild visible rows without cloning the source's scrollback or other grid.
pub(crate) fn capture(screen: &vt100::Screen, rows: u16) -> Result<vt100::Screen, SelectionError> {
    let (_, columns) = screen.size();
    if usize::from(rows) * usize::from(columns) > MAX_SELECTION_CELLS {
        return Err(SelectionError::CaptureLimit);
    }
    let mut frozen = vt100::Parser::new(rows, columns, usize::default());
    for (row, contents) in screen
        .rows_formatted(0, columns)
        .take(usize::from(rows))
        .enumerate()
    {
        let position = row + usize::from(CELL_STEP);
        frozen.process(format!("{ROW_PREFIX}{position}{ROW_SUFFIX}").as_bytes());
        frozen.process(&contents);
    }
    frozen.process(HIDE_CURSOR);
    Ok(frozen.screen().clone())
}

use crate::{CellPoint, Selection, SelectionError, constants::*};

impl Selection {
    /// Return immutable selected text; partial or over-limit copies fail whole.
    pub fn text(&self) -> Result<String, SelectionError> {
        if self.is_dragging() {
            return Err(SelectionError::Incomplete);
        }
        let (start, end) = self.range();
        let mut text = String::new();
        for row in start.row..=end.row {
            let first = if row == start.row { start.column } else { 0 };
            let last = if row == end.row {
                end.column
            } else {
                self.snapshot.columns - CELL_STEP
            };
            let mut blanks = usize::default();
            for column in first..=last {
                let cell = self.snapshot.cell(CellPoint { row, column });
                if cell.is_wide_continuation() {
                    continue;
                }
                if !cell.has_contents()
                    || (cell.is_wide() && column + CELL_STEP == self.snapshot.columns)
                {
                    blanks += usize::from(CELL_STEP);
                    continue;
                }
                while blanks > 0 {
                    append(&mut text, SPACE)?;
                    blanks -= usize::from(CELL_STEP);
                }
                append(&mut text, cell.contents())?;
            }
            // Screen padding is omitted; a physical soft wrap joins its rows.
            if row != end.row && !self.snapshot.wrapped(row) {
                append(&mut text, NEWLINE)?;
            }
        }
        if text.is_empty() {
            return Err(SelectionError::EmptySelection);
        }
        Ok(text)
    }
}

fn append(text: &mut String, value: &str) -> Result<(), SelectionError> {
    if value.len() > MAX_SELECTION_BYTES.saturating_sub(text.len()) {
        return Err(SelectionError::CopyLimit);
    }
    text.push_str(value);
    Ok(())
}

use crate::{
    CellPoint, MAX_SELECTION_CELLS, SelectionError,
    constants::{BLANK_CELL_EXPECTED, CELL_STEP},
};

#[derive(Debug)]
pub(crate) struct Snapshot {
    pub rows: u16,
    pub columns: u16,
    source_size: (u16, u16),
    cells: Vec<vt100::Cell>,
    wrapped: Vec<bool>,
}

impl Snapshot {
    pub fn new(screen: &vt100::Screen, rows: u16, columns: u16) -> Result<Self, SelectionError> {
        if rows == 0 || columns == 0 {
            return Err(SelectionError::EmptyViewport);
        }
        let source_size = screen.size();
        if rows > source_size.0 || columns > source_size.1 {
            return Err(SelectionError::InvalidPoint);
        }
        let count = usize::from(rows) * usize::from(columns);
        if count > MAX_SELECTION_CELLS {
            return Err(SelectionError::CaptureLimit);
        }
        let mut cells = Vec::with_capacity(count);
        let mut wrapped = Vec::with_capacity(usize::from(rows));
        for row in 0..rows {
            wrapped.push(columns == source_size.1 && screen.row_wrapped(row));
            for column in 0..columns {
                // Resized scrollback can retain narrower historical rows.
                cells.push(screen.cell(row, column).unwrap_or_else(|| blank()).clone());
            }
        }
        Ok(Self {
            rows,
            columns,
            source_size,
            cells,
            wrapped,
        })
    }

    pub fn contains(&self, point: CellPoint) -> bool {
        point.row < self.rows && point.column < self.columns
    }

    pub fn cell(&self, point: CellPoint) -> &vt100::Cell {
        &self.cells[usize::from(point.row) * usize::from(self.columns) + usize::from(point.column)]
    }

    pub fn wrapped(&self, row: u16) -> bool {
        self.wrapped[usize::from(row)]
    }

    pub fn matches(&self, screen: &vt100::Screen) -> bool {
        self.source_size == screen.size()
            && (0..self.rows).all(|row| {
                self.wrapped(row) == (self.columns == self.source_size.1 && screen.row_wrapped(row))
                    && (0..self.columns).all(|column| {
                        screen.cell(row, column).unwrap_or_else(|| blank())
                            == self.cell(CellPoint { row, column })
                    })
            })
    }
}

fn blank() -> &'static vt100::Cell {
    static CELL: std::sync::OnceLock<vt100::Cell> = std::sync::OnceLock::new();
    CELL.get_or_init(|| {
        vt100::Parser::new(CELL_STEP, CELL_STEP, usize::default())
            .screen()
            .cell(0, 0)
            .expect(BLANK_CELL_EXPECTED)
            .clone()
    })
}

use crate::{SelectionError, constants::CELL_STEP, snapshot::Snapshot};

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct CellPoint {
    pub row: u16,
    pub column: u16,
}

#[derive(Debug)]
pub struct Selection {
    pub(crate) snapshot: Snapshot,
    frozen: vt100::Screen,
    anchor: CellPoint,
    head: CellPoint,
    dragging: bool,
}

impl Selection {
    /// Freeze only the displayed rectangle, using vt100's visible-cell API.
    pub fn begin(
        screen: &vt100::Screen,
        rows: u16,
        columns: u16,
        anchor: CellPoint,
    ) -> Result<Self, SelectionError> {
        let snapshot = Snapshot::new(screen, rows, columns)?;
        if !snapshot.contains(anchor) {
            return Err(SelectionError::InvalidPoint);
        }
        let frozen = crate::frozen::capture(screen, rows)?;
        Ok(Self {
            snapshot,
            frozen,
            anchor,
            head: anchor,
            dragging: true,
        })
    }

    pub fn extend(&mut self, head: CellPoint) -> Result<(), SelectionError> {
        if !self.dragging || !self.snapshot.contains(head) {
            return Err(SelectionError::InvalidPoint);
        }
        self.head = head;
        Ok(())
    }

    pub fn finish(&mut self, head: CellPoint) -> Result<(), SelectionError> {
        self.extend(head)?;
        self.dragging = false;
        Ok(())
    }

    pub fn is_dragging(&self) -> bool {
        self.dragging
    }

    pub fn dimensions(&self) -> (u16, u16) {
        (self.snapshot.rows, self.snapshot.columns)
    }

    pub fn screen(&self) -> &vt100::Screen {
        &self.frozen
    }

    /// Ordered inclusive endpoints, expanded to include whole wide characters.
    pub fn range(&self) -> (CellPoint, CellPoint) {
        let (mut start, mut end) = if self.anchor <= self.head {
            (self.anchor, self.head)
        } else {
            (self.head, self.anchor)
        };
        if self.snapshot.cell(start).is_wide_continuation() {
            start.column = start.column.saturating_sub(CELL_STEP);
        }
        if self.snapshot.cell(end).is_wide() {
            end.column = (end.column + CELL_STEP).min(self.snapshot.columns - CELL_STEP);
        }
        (start, end)
    }

    pub fn contains(&self, point: CellPoint) -> bool {
        let (start, end) = self.range();
        self.snapshot.contains(point) && point >= start && point <= end
    }

    /// Detect a replaced snapshot even when its output cursor was unchanged.
    pub fn matches(&self, screen: &vt100::Screen) -> bool {
        self.snapshot.matches(screen)
    }
}

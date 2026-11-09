use crate::{Axis, Layout, LayoutError, RATIO_SCALE, SplitRatio, Tree};
use uuid::Uuid;

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct Rect {
    pub x: u16,
    pub y: u16,
    pub width: u16,
    pub height: u16,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PaneRect {
    pub pane: Uuid,
    pub rect: Rect,
}

impl Layout {
    pub fn geometry(&self, bounds: Rect) -> Result<Vec<PaneRect>, LayoutError> {
        bounds.validate()?;
        let mut result = Vec::new();
        place(&self.0.root, bounds, &mut result);
        Ok(result)
    }
}

fn place(tree: &Tree, rect: Rect, output: &mut Vec<PaneRect>) {
    match tree {
        Tree::Pane(pane) => output.push(PaneRect {
            pane: pane.id,
            rect,
        }),
        Tree::Split {
            axis,
            ratio,
            first,
            second,
        } => {
            let (a, b) = rect.partition(*axis, *ratio);
            place(first, a, output);
            place(second, b, output);
        }
    }
}

impl Rect {
    pub(crate) fn validate(self) -> Result<(), LayoutError> {
        if self.x.checked_add(self.width).is_none() || self.y.checked_add(self.height).is_none() {
            Err(LayoutError::InvalidRect)
        } else {
            Ok(())
        }
    }

    /// Callers validate the outer rectangle before descending into partitions.
    pub(crate) fn partition(self, axis: Axis, ratio: SplitRatio) -> (Self, Self) {
        let span = match axis {
            Axis::Horizontal => self.width,
            Axis::Vertical => self.height,
        };
        let extent =
            (u32::from(span) * u32::from(u16::from(ratio)) / u32::from(RATIO_SCALE)) as u16;
        match axis {
            Axis::Horizontal => (
                Self {
                    width: extent,
                    ..self
                },
                Self {
                    x: self.x + extent,
                    width: span - extent,
                    ..self
                },
            ),
            Axis::Vertical => (
                Self {
                    height: extent,
                    ..self
                },
                Self {
                    y: self.y + extent,
                    height: span - extent,
                    ..self
                },
            ),
        }
    }
}

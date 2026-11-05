use crate::{Layout, LayoutError, Rect};
use uuid::Uuid;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Direction {
    Left,
    Right,
    Up,
    Down,
}

impl Layout {
    /// Prefer an aligned neighbor, then nearest directional edge and center.
    /// At an outside edge focus stays put; zero-area panes are never targets.
    pub fn focus_direction(
        &mut self,
        direction: Direction,
        bounds: Rect,
    ) -> Result<bool, LayoutError> {
        let geometry = self.geometry(bounds)?;
        let current = geometry
            .iter()
            .find(|pane| pane.pane == self.0.focused)
            .ok_or(LayoutError::UnknownPane)?
            .rect;
        if !visible(current) {
            return Ok(false);
        }
        let next = geometry
            .iter()
            .filter(|pane| pane.pane != self.0.focused && visible(pane.rect))
            .filter_map(|pane| score(direction, current, pane.rect, pane.pane))
            .min()
            .map(|score| score.3);
        if let Some(next) = next {
            self.0.focused = next;
        }
        Ok(next.is_some())
    }
}

fn visible(rect: Rect) -> bool {
    rect.width != 0 && rect.height != 0
}

fn score(
    direction: Direction,
    current: Rect,
    candidate: Rect,
    id: Uuid,
) -> Option<(bool, u16, u32, Uuid)> {
    let (distance, a_start, a_end, b_start, b_end) = match direction {
        Direction::Left => (
            current.x.checked_sub(candidate.x + candidate.width)?,
            current.y,
            current.y + current.height,
            candidate.y,
            candidate.y + candidate.height,
        ),
        Direction::Right => (
            candidate.x.checked_sub(current.x + current.width)?,
            current.y,
            current.y + current.height,
            candidate.y,
            candidate.y + candidate.height,
        ),
        Direction::Up => (
            current.y.checked_sub(candidate.y + candidate.height)?,
            current.x,
            current.x + current.width,
            candidate.x,
            candidate.x + candidate.width,
        ),
        Direction::Down => (
            candidate.y.checked_sub(current.y + current.height)?,
            current.x,
            current.x + current.width,
            candidate.x,
            candidate.x + candidate.width,
        ),
    };
    let aligned = a_start < b_end && b_start < a_end;
    let centers =
        (u32::from(a_start) + u32::from(a_end)).abs_diff(u32::from(b_start) + u32::from(b_end));
    Some((!aligned, distance, centers, id))
}

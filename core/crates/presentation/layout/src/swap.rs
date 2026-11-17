use crate::{Layout, LayoutError, Pane, Tree, constants::*};
use uuid::Uuid;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SwapDirection {
    Previous,
    Next,
}

impl Layout {
    /// Exchanges leaf positions without changing pane bindings, focus, ratios or
    /// split topology. Unknown identities fail before any tree mutation.
    pub fn swap_panes(&mut self, first: Uuid, second: Uuid) -> Result<bool, LayoutError> {
        let panes = self.panes();
        let first = panes
            .iter()
            .find(|pane| pane.id == first)
            .map(|pane| (**pane).clone())
            .ok_or(LayoutError::UnknownPane)?;
        let second = panes
            .iter()
            .find(|pane| pane.id == second)
            .map(|pane| (**pane).clone())
            .ok_or(LayoutError::UnknownPane)?;
        if first.id == second.id {
            return Ok(false);
        }
        exchange(&mut self.0.root, &first, &second);
        Ok(true)
    }

    /// Previous/next follow depth-first visual leaf order and wrap at either end.
    pub fn swap_focused(&mut self, direction: SwapDirection) -> Result<bool, LayoutError> {
        let panes = self.panes();
        if panes.len() == SINGLE_PANE {
            return Ok(false);
        }
        let current = panes
            .iter()
            .position(|pane| pane.id == self.focused())
            .expect(FOCUSED_PANE_INVARIANT);
        let neighbor = match direction {
            SwapDirection::Next => (current + NEIGHBOR_STEP) % panes.len(),
            SwapDirection::Previous => (current + panes.len() - NEIGHBOR_STEP) % panes.len(),
        };
        let next = panes[neighbor].id;
        self.swap_panes(self.focused(), next)
    }
}

fn exchange(tree: &mut Tree, first: &Pane, second: &Pane) {
    match tree {
        Tree::Pane(pane) if pane.id == first.id => *pane = second.clone(),
        Tree::Pane(pane) if pane.id == second.id => *pane = first.clone(),
        Tree::Pane(_) => {}
        Tree::Split {
            first: left,
            second: right,
            ..
        } => {
            exchange(left, first, second);
            exchange(right, first, second);
        }
    }
}

use crate::split_path::{focused_path, tree_at_mut};
use crate::{Axis, Layout, LayoutError, MAX_RATIO, MIN_RATIO, SplitBranch, SplitRatio, Tree};

impl Layout {
    pub fn resize_split(
        &mut self,
        path: &[SplitBranch],
        ratio: SplitRatio,
    ) -> Result<bool, LayoutError> {
        let Tree::Split { ratio: current, .. } = tree_at_mut(&mut self.0.root, path)? else {
            return Err(LayoutError::InvalidSplitPath);
        };
        let changed = *current != ratio;
        *current = ratio;
        Ok(changed)
    }

    /// Adjust only the nearest matching-axis ancestor, even when it is clamped.
    pub fn adjust_focused(
        &mut self,
        axis: Axis,
        grow: bool,
        step: u16,
    ) -> Result<bool, LayoutError> {
        let mut path =
            focused_path(&self.0.root, self.0.focused).ok_or(LayoutError::UnknownPane)?;
        while let Some(branch) = path.pop() {
            let Tree::Split {
                axis: split_axis,
                ratio,
                ..
            } = tree_at_mut(&mut self.0.root, &path)?
            else {
                return Err(LayoutError::InvalidSplitPath);
            };
            if *split_axis == axis {
                let previous = u16::from(*ratio);
                let next = if grow == (branch == SplitBranch::First) {
                    previous.saturating_add(step).min(MAX_RATIO)
                } else {
                    previous.saturating_sub(step).max(MIN_RATIO)
                };
                *ratio = SplitRatio::try_from(next)?;
                return Ok(previous != next);
            }
        }
        Ok(false)
    }

    pub fn balance(&mut self) -> bool {
        balance(&mut self.0.root)
    }
}

fn balance(tree: &mut Tree) -> bool {
    let Tree::Split {
        ratio,
        first,
        second,
        ..
    } = tree
    else {
        return false;
    };
    let first_changed = balance(first);
    let second_changed = balance(second);
    let changed = *ratio != SplitRatio::default();
    *ratio = SplitRatio::default();
    changed || first_changed || second_changed
}

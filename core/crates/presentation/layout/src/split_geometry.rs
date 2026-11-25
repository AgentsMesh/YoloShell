use crate::{Axis, Layout, LayoutError, Rect, SplitRatio, Tree};

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub enum SplitBranch {
    First,
    Second,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SplitGeometry {
    pub path: Vec<SplitBranch>,
    pub axis: Axis,
    pub rect: Rect,
    /// Absolute x for a horizontal split, or absolute y for a vertical split.
    pub boundary: u16,
    pub ratio: SplitRatio,
}

impl Layout {
    pub fn split_geometry(&self, bounds: Rect) -> Result<Vec<SplitGeometry>, LayoutError> {
        bounds.validate()?;
        let mut result = Vec::new();
        collect(&self.0.root, bounds, &mut Vec::new(), &mut result);
        Ok(result)
    }
}

fn collect(tree: &Tree, rect: Rect, path: &mut Vec<SplitBranch>, output: &mut Vec<SplitGeometry>) {
    let Tree::Split {
        axis,
        ratio,
        first,
        second,
    } = tree
    else {
        return;
    };
    let (a, b) = rect.partition(*axis, *ratio);
    output.push(SplitGeometry {
        path: path.clone(),
        axis: *axis,
        rect,
        boundary: match axis {
            Axis::Horizontal => b.x,
            Axis::Vertical => b.y,
        },
        ratio: *ratio,
    });
    path.push(SplitBranch::First);
    collect(first, a, path, output);
    path.pop();
    path.push(SplitBranch::Second);
    collect(second, b, path, output);
    path.pop();
}

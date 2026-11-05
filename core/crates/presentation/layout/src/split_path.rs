use crate::{LayoutError, MAX_LAYOUT_DEPTH, SplitBranch, Tree};
use uuid::Uuid;

pub(crate) fn tree_at_mut<'a>(
    tree: &'a mut Tree,
    path: &[SplitBranch],
) -> Result<&'a mut Tree, LayoutError> {
    if path.len() > MAX_LAYOUT_DEPTH {
        return Err(LayoutError::InvalidSplitPath);
    }
    let mut current = tree;
    for branch in path {
        let Tree::Split { first, second, .. } = current else {
            return Err(LayoutError::InvalidSplitPath);
        };
        current = match branch {
            SplitBranch::First => first,
            SplitBranch::Second => second,
        };
    }
    Ok(current)
}

pub(crate) fn focused_path(tree: &Tree, focused: Uuid) -> Option<Vec<SplitBranch>> {
    let mut path = Vec::new();
    find(tree, focused, &mut path).then_some(path)
}

fn find(tree: &Tree, focused: Uuid, path: &mut Vec<SplitBranch>) -> bool {
    match tree {
        Tree::Pane(pane) => pane.id == focused,
        Tree::Split { first, second, .. } => {
            for (branch, subtree) in [(SplitBranch::First, first), (SplitBranch::Second, second)] {
                path.push(branch);
                if find(subtree, focused, path) {
                    return true;
                }
                path.pop();
            }
            false
        }
    }
}

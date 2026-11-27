use crate::constants::{CHILD_DEPTH, MAX_LAYOUT_DEPTH, MAX_PANES, ROOT_DEPTH};
use crate::{Layout, PaneLayoutDocument, LayoutError, Tree};
use std::collections::BTreeSet;
use uuid::Uuid;

impl Tree {
    pub fn validate(&self) -> Result<(), LayoutError> {
        validate(self, ROOT_DEPTH, &mut BTreeSet::new())
    }
}

impl TryFrom<PaneLayoutDocument> for Layout {
    type Error = LayoutError;
    fn try_from(document: PaneLayoutDocument) -> Result<Self, Self::Error> {
        let mut identities = BTreeSet::new();
        validate(&document.root, ROOT_DEPTH, &mut identities)?;
        if !identities.contains(&document.focused) {
            return Err(LayoutError::UnknownPane);
        }
        Ok(Self(document))
    }
}

fn validate(tree: &Tree, depth: usize, identities: &mut BTreeSet<Uuid>) -> Result<(), LayoutError> {
    if depth > MAX_LAYOUT_DEPTH {
        return Err(LayoutError::Limit);
    }
    match tree {
        Tree::Pane(pane) => {
            if pane.id.is_nil() || !identities.insert(pane.id) {
                return Err(LayoutError::InvalidId);
            }
            if identities.len() > MAX_PANES {
                return Err(LayoutError::Limit);
            }
        }
        Tree::Split { first, second, .. } => {
            validate(first, depth + CHILD_DEPTH, identities)?;
            validate(second, depth + CHILD_DEPTH, identities)?;
        }
    }
    Ok(())
}

use crate::{Axis, Layout, LayoutError, Pane, SplitRatio, Tree};
use uuid::Uuid;
use yolo_hub_identity::TerminalAddress;

impl Layout {
    pub fn focus(&mut self, id: Uuid) -> Result<(), LayoutError> {
        if !self.panes().iter().any(|pane| pane.id == id) {
            return Err(LayoutError::UnknownPane);
        }
        self.0.focused = id;
        Ok(())
    }

    pub fn bind(&mut self, id: Uuid, terminal: Option<TerminalAddress>) -> Result<(), LayoutError> {
        let pane = self.0.root.pane_mut(id).ok_or(LayoutError::UnknownPane)?;
        pane.terminal = terminal;
        Ok(())
    }

    pub fn split(
        &mut self,
        axis: Axis,
        terminal: Option<TerminalAddress>,
    ) -> Result<Uuid, LayoutError> {
        let id = Uuid::new_v4();
        let mut next = self.0.clone();
        let target = next
            .root
            .find_mut(next.focused)
            .ok_or(LayoutError::UnknownPane)?;
        *target = Tree::Split {
            axis,
            ratio: SplitRatio::default(),
            first: Box::new(target.clone()),
            second: Box::new(Tree::Pane(Pane { id, terminal })),
        };
        next.focused = id;
        *self = Self::try_from(next)?;
        Ok(id)
    }

    pub fn resize_parent(&mut self, pane: Uuid, ratio: SplitRatio) -> Result<(), LayoutError> {
        if !self.0.root.resize_parent(pane, ratio) {
            return Err(LayoutError::UnknownPane);
        }
        Ok(())
    }

    pub fn close(&mut self, id: Uuid) -> Result<(), LayoutError> {
        let panes = self.panes();
        if !panes.iter().any(|pane| pane.id == id) {
            return Err(LayoutError::UnknownPane);
        }
        let fallback = panes
            .iter()
            .find(|pane| pane.id != id)
            .ok_or(LayoutError::LastPane)?
            .id;
        let mut next = self.0.clone();
        next.root = next.root.remove(id).ok_or(LayoutError::LastPane)?;
        if next.focused == id {
            next.focused = fallback;
        }
        *self = Self::try_from(next)?;
        Ok(())
    }
}

impl Tree {
    fn pane_mut(&mut self, id: Uuid) -> Option<&mut Pane> {
        match self.find_mut(id)? {
            Self::Pane(pane) => Some(pane),
            Self::Split { .. } => None,
        }
    }

    fn find_mut(&mut self, id: Uuid) -> Option<&mut Self> {
        match self {
            Self::Pane(pane) if pane.id == id => Some(self),
            Self::Pane(_) => None,
            Self::Split { first, second, .. } => first.find_mut(id).or_else(|| second.find_mut(id)),
        }
    }

    fn resize_parent(&mut self, pane: Uuid, value: SplitRatio) -> bool {
        let Self::Split {
            ratio,
            first,
            second,
            ..
        } = self
        else {
            return false;
        };
        if first.is_pane(pane) || second.is_pane(pane) {
            *ratio = value;
            return true;
        }
        first.resize_parent(pane, value) || second.resize_parent(pane, value)
    }

    fn is_pane(&self, id: Uuid) -> bool {
        matches!(self, Self::Pane(pane) if pane.id == id)
    }

    fn remove(self, id: Uuid) -> Option<Self> {
        match self {
            Self::Pane(pane) => (pane.id != id).then_some(Self::Pane(pane)),
            Self::Split {
                axis,
                ratio,
                first,
                second,
            } => match (first.remove(id), second.remove(id)) {
                (Some(first), Some(second)) => Some(Self::Split {
                    axis,
                    ratio,
                    first: Box::new(first),
                    second: Box::new(second),
                }),
                (first, second) => first.or(second),
            },
        }
    }
}

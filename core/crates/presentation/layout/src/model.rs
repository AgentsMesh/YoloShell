use crate::LayoutError;
use crate::constants::{BALANCED_RATIO, MAX_RATIO, MIN_RATIO};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use yolo_hub_identity::TerminalAddress;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Axis {
    Horizontal,
    Vertical,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(try_from = "u16", into = "u16")]
pub struct SplitRatio(u16);

impl TryFrom<u16> for SplitRatio {
    type Error = LayoutError;
    fn try_from(value: u16) -> Result<Self, Self::Error> {
        if (MIN_RATIO..=MAX_RATIO).contains(&value) {
            Ok(Self(value))
        } else {
            Err(LayoutError::InvalidRatio)
        }
    }
}

impl From<SplitRatio> for u16 {
    fn from(value: SplitRatio) -> Self {
        value.0
    }
}

impl Default for SplitRatio {
    fn default() -> Self {
        Self(BALANCED_RATIO)
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Pane {
    #[serde(with = "crate::identity")]
    pub id: Uuid,
    pub terminal: Option<TerminalAddress>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum Tree {
    Pane(Pane),
    Split {
        axis: Axis,
        ratio: SplitRatio,
        first: Box<Tree>,
        second: Box<Tree>,
    },
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PaneLayoutDocument {
    pub root: Tree,
    /// Selection belongs to one client; shared documents persist only the tree.
    #[serde(with = "crate::identity")]
    pub focused: Uuid,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(try_from = "PaneLayoutDocument", into = "PaneLayoutDocument")]
pub struct Layout(pub(crate) PaneLayoutDocument);

impl Layout {
    pub fn new(terminal: Option<TerminalAddress>) -> Self {
        let id = Uuid::new_v4();
        Self(PaneLayoutDocument {
            root: Tree::Pane(Pane { id, terminal }),
            focused: id,
        })
    }

    pub fn document(&self) -> &PaneLayoutDocument {
        &self.0
    }
    pub fn focused(&self) -> Uuid {
        self.0.focused
    }
    pub fn panes(&self) -> Vec<&Pane> {
        let mut result = Vec::new();
        self.0.root.collect(&mut result);
        result
    }
}

impl Default for Layout {
    fn default() -> Self {
        Self::new(None)
    }
}

impl From<Layout> for PaneLayoutDocument {
    fn from(value: Layout) -> Self {
        value.0
    }
}

impl Tree {
    pub fn panes(&self) -> Vec<&Pane> {
        let mut panes = Vec::new();
        self.collect(&mut panes);
        panes
    }

    fn collect<'a>(&'a self, panes: &mut Vec<&'a Pane>) {
        match self {
            Self::Pane(pane) => panes.push(pane),
            Self::Split { first, second, .. } => {
                first.collect(panes);
                second.collect(panes);
            }
        }
    }
}

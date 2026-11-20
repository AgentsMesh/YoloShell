use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Right {
    Observe,
    TerminalRead,
    TerminalWrite,
    Lifecycle,
    FileRead,
    AgentControl,
    ConversationRead,
    PresentationRead,
    PresentationWrite,
    WorkspaceRead,
    WorkspaceWrite,
}

/// Rights are independent: terminal writing does not imply semantic Agent control.
#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Rights(BTreeSet<Right>);

impl Rights {
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    pub fn allows(&self, required: &Self) -> bool {
        required.0.is_subset(&self.0)
    }

    pub fn intersection(&self, other: &Self) -> Self {
        Self(self.0.intersection(&other.0).copied().collect())
    }

    pub fn union(&self, other: &Self) -> Self {
        Self(self.0.union(&other.0).copied().collect())
    }
}

impl FromIterator<Right> for Rights {
    fn from_iter<T: IntoIterator<Item = Right>>(iter: T) -> Self {
        Self(iter.into_iter().collect())
    }
}

use crate::{ProtocolError, validation};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use yolo_presentation_layout::Tree;

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(try_from = "TabFields")]
pub struct SharedTab {
    #[serde(with = "crate::identity")]
    pub id: Uuid,
    pub title: String,
    pub root: Tree,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct TabFields {
    #[serde(with = "crate::identity")]
    id: Uuid,
    title: String,
    root: Tree,
}

impl SharedTab {
    pub fn new(id: Uuid, title: String, root: Tree) -> Result<Self, ProtocolError> {
        let tab = Self { id, title, root };
        validation::shared_tab(&tab)?;
        Ok(tab)
    }
}
impl TryFrom<TabFields> for SharedTab {
    type Error = ProtocolError;
    fn try_from(fields: TabFields) -> Result<Self, Self::Error> {
        Self::new(fields.id, fields.title, fields.root)
    }
}

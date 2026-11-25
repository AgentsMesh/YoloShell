use crate::{TabCatalog, constants::*};
use anyhow::{Result, ensure};
use serde::{Deserialize, Serialize};
use yolo_hub_identity::HubId;
use yolo_presentation_layout::Layout;

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TabViewMode {
    #[default]
    Inventory,
    Terminal,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DocumentPane {
    pub title: String,
    pub text: String,
}

/// Only stable UI state belongs here. A restored view never restores writer authority.
#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LocalTabView {
    pub layout: Layout,
    pub view: TabViewMode,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TabDocument {
    pub schema_version: u16,
    pub entry_hub: HubId,
    pub layout_address: yolo_presentation_protocol::LayoutAddress,
    pub shared: Option<LayoutCheckpoint>,
    pub revision: u64,
    pub tabs: TabCatalog,
}
impl TabDocument {
    pub fn validate(&self, entry: HubId) -> Result<()> {
        ensure!(
            self.schema_version == TAB_SCHEMA_VERSION,
            SCHEMA_ERROR
        );
        ensure!(self.revision >= FIRST_TAB_REVISION, REVISION_ERROR);
        ensure!(self.entry_hub == entry, ENTRY_ERROR);
        ensure!(self.layout_address.hub_id == entry, ENTRY_ERROR);
        ensure!(!self.layout_address.layout_id.is_nil(), ID_ERROR);
        if let Some(shared) = &self.shared {
            shared.document.validate()?;
            ensure!(shared.document.address == self.layout_address, ENTRY_ERROR);
            let projected = yolo_presentation_protocol::LayoutUpdate {
                address: self.layout_address,
                request_id: shared.document.last_request_id,
                expected_revision: 0,
                tabs: shared.projected.clone(),
            };
            projected.validate()?;
            ensure!(
                shared.document.tabs.is_empty() || shared.document.tabs == shared.projected,
                ID_ERROR
            );
        }
        Ok(())
    }
}

/// Last acknowledged public revision plus its local placeholder projection.
/// Private topology may be a newer draft; losing a reply never destroys that draft.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LayoutCheckpoint {
    pub document: yolo_presentation_protocol::SharedLayoutDocument,
    pub projected: Vec<yolo_presentation_protocol::SharedTab>,
}

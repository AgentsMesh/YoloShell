use crate::constants::*;
use anyhow::{Result, ensure};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use yolo_hub_identity::HubId;
use yolo_presentation_protocol::SharedLayoutDocument;

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Record {
    pub schema_version: u16,
    pub hub_id: HubId,
    pub documents: Vec<SharedLayoutDocument>,
}

impl Record {
    pub fn empty(hub_id: HubId) -> Self {
        Self {
            schema_version: SCHEMA_VERSION,
            hub_id,
            documents: Vec::new(),
        }
    }
    pub fn validate(&self, hub_id: HubId) -> Result<()> {
        ensure!(
            self.schema_version == SCHEMA_VERSION && self.hub_id == hub_id,
            RECORD_INVALID
        );
        ensure!(self.documents.len() <= MAX_DOCUMENTS, RECORD_INVALID);
        let mut ids = BTreeSet::new();
        for document in &self.documents {
            document.validate()?;
            ensure!(
                document.address.hub_id == hub_id && ids.insert(document.address.layout_id),
                RECORD_INVALID
            );
        }
        ensure!(
            // Match the durable writer's encoding before admission can retire a healthy store.
            u64::try_from(serde_json::to_vec_pretty(self)?.len())?
                <= yolo_paths::constants::MAX_DURABLE_JSON_BYTES,
            TOO_LARGE
        );
        Ok(())
    }
}

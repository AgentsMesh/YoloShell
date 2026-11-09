use crate::constants::*;
use anyhow::{Result, ensure};
use serde::{Deserialize, Serialize};
use yolo_agent_control_journal::{JournalAnchor, JournalScope};
use yolo_application_support::constants::RUNTIME_RECORD_SCHEMA_VERSION;
use yolo_terminal_model::RuntimeRecord;

// The runtime contains its authentication token. Never derive Debug or expose it.
#[derive(Clone, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct AnchorRecord {
    pub schema: u16,
    pub runtime: RuntimeRecord,
    pub journal: JournalAnchor,
}

impl AnchorRecord {
    pub fn validate(&self, current: &RuntimeRecord) -> Result<()> {
        ensure!(
            self.schema == SCHEMA && self.runtime.schema_version == RUNTIME_RECORD_SCHEMA_VERSION,
            INVALID_ANCHOR
        );
        let scope = JournalScope::new(self.runtime.terminal_id, self.runtime.runtime_epoch)?;
        ensure!(self.journal.scope() == scope, INVALID_ANCHOR);
        current.validate_update_from(&self.runtime)?;
        Ok(())
    }
}

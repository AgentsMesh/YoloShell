use crate::constants::INVALID_HISTORY;
use anyhow::{Result, ensure};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use yolo_hub_identity::HubId;

/// Last durable checkpoint, not a claim about current connectivity or progress.
// Serde requires a literal tag; its public name is SETUP_CHECKPOINT_PHASE_TAG.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(tag = "phase", rename_all = "snake_case", deny_unknown_fields)]
pub enum SetupCheckpoint {
    Prepared {},
    Provisioning {},
    Routing { origin: HubId },
    Complete { origin: HubId },
}

/// A private-path-free projection of one entry Hub's durable setup journal.
/// `None` means no record was found; it grants no permission to replay anything.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(try_from = "HistoryWire", into = "HistoryWire")]
pub struct RemoteSetupHistory {
    pub operation_id: Uuid,
    pub owner: HubId,
    pub checkpoint: Option<SetupCheckpoint>,
}

impl RemoteSetupHistory {
    pub fn new(
        operation_id: Uuid,
        owner: HubId,
        checkpoint: Option<SetupCheckpoint>,
    ) -> Result<Self> {
        let history = Self {
            operation_id,
            owner,
            checkpoint,
        };
        history.validate_for(owner, operation_id)?;
        Ok(history)
    }

    pub fn validate_for(&self, owner: HubId, operation_id: Uuid) -> Result<()> {
        ensure!(
            !self.operation_id.is_nil() && self.operation_id == operation_id && self.owner == owner,
            INVALID_HISTORY
        );
        if let Some(SetupCheckpoint::Routing { origin } | SetupCheckpoint::Complete { origin }) =
            self.checkpoint
        {
            ensure!(origin != self.owner, INVALID_HISTORY);
        }
        Ok(())
    }
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct HistoryWire {
    operation_id: Uuid,
    owner: HubId,
    checkpoint: Option<SetupCheckpoint>,
}

impl TryFrom<HistoryWire> for RemoteSetupHistory {
    type Error = anyhow::Error;

    fn try_from(value: HistoryWire) -> Result<Self> {
        Self::new(value.operation_id, value.owner, value.checkpoint)
    }
}

impl From<RemoteSetupHistory> for HistoryWire {
    fn from(value: RemoteSetupHistory) -> Self {
        Self {
            operation_id: value.operation_id,
            owner: value.owner,
            checkpoint: value.checkpoint,
        }
    }
}

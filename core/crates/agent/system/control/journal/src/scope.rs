use crate::{JournalError, constants::INVALID_SCOPE};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(try_from = "ScopeWire")]
pub struct JournalScope {
    #[serde(with = "yolo_crypto::serde_non_nil_uuid")]
    terminal_id: Uuid,
    #[serde(with = "yolo_crypto::serde_non_nil_uuid")]
    runtime_epoch: Uuid,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ScopeWire {
    #[serde(with = "yolo_crypto::serde_non_nil_uuid")]
    terminal_id: Uuid,
    #[serde(with = "yolo_crypto::serde_non_nil_uuid")]
    runtime_epoch: Uuid,
}
impl JournalScope {
    pub fn new(terminal_id: Uuid, runtime_epoch: Uuid) -> Result<Self, JournalError> {
        if terminal_id.is_nil() || runtime_epoch.is_nil() {
            return Err(JournalError::InvalidRequest);
        }
        Ok(Self {
            terminal_id,
            runtime_epoch,
        })
    }
    pub fn terminal_id(self) -> Uuid {
        self.terminal_id
    }
    pub fn runtime_epoch(self) -> Uuid {
        self.runtime_epoch
    }
}
impl TryFrom<ScopeWire> for JournalScope {
    type Error = &'static str;
    fn try_from(wire: ScopeWire) -> Result<Self, Self::Error> {
        Self::new(wire.terminal_id, wire.runtime_epoch).map_err(|_| INVALID_SCOPE)
    }
}

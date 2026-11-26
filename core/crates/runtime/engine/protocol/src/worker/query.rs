use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SnapshotRequest {
    #[serde(with = "yolo_crypto::serde_non_nil_uuid")]
    pub terminal_id: Uuid,
    #[serde(with = "yolo_crypto::serde_non_nil_uuid")]
    pub runtime_epoch: Uuid,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TerminalTextRequest {
    #[serde(with = "yolo_crypto::serde_non_nil_uuid")]
    pub terminal_id: Uuid,
    #[serde(with = "yolo_crypto::serde_non_nil_uuid")]
    pub runtime_epoch: Uuid,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TerminalTextResult {
    #[serde(with = "yolo_crypto::serde_non_nil_uuid")]
    pub runtime_epoch: Uuid,
    pub sequence: u64,
    pub cols: u16,
    pub rows: u16,
    pub text: String,
    pub truncated: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RuntimeRef {
    #[serde(with = "yolo_crypto::serde_non_nil_uuid")]
    pub terminal_id: Uuid,
    #[serde(with = "yolo_crypto::serde_non_nil_uuid")]
    pub runtime_epoch: Uuid,
}

impl RuntimeRef {
    pub const fn new(terminal_id: Uuid, runtime_epoch: Uuid) -> Self {
        Self {
            terminal_id,
            runtime_epoch,
        }
    }
}

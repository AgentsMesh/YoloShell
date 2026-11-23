use serde::{Deserialize, Serialize};
use uuid::Uuid;
use yolo_terminal_model::{TerminalRuntimeStatus, WriterCapability};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FencedTerminateRequest {
    #[serde(with = "yolo_crypto::serde_non_nil_uuid")]
    pub terminal_id: Uuid,
    #[serde(with = "yolo_crypto::serde_non_nil_uuid")]
    pub runtime_epoch: Uuid,
    pub expected_status: Option<TerminalRuntimeStatus>,
    pub force: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct InputRequest {
    #[serde(with = "yolo_crypto::serde_non_nil_uuid")]
    pub terminal_id: Uuid,
    #[serde(with = "yolo_crypto::serde_non_nil_uuid")]
    pub runtime_epoch: Uuid,
    pub generation: u64,
    pub writer_capability: WriterCapability,
    #[serde(with = "yolo_crypto::serde_uuid")]
    pub input_id: Uuid,
    #[serde(with = "serde_bytes")]
    pub data: Vec<u8>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ResizeRequest {
    #[serde(with = "yolo_crypto::serde_non_nil_uuid")]
    pub terminal_id: Uuid,
    #[serde(with = "yolo_crypto::serde_non_nil_uuid")]
    pub runtime_epoch: Uuid,
    pub generation: u64,
    pub writer_capability: WriterCapability,
    pub cols: u16,
    pub rows: u16,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SignalRequest {
    #[serde(with = "yolo_crypto::serde_non_nil_uuid")]
    pub terminal_id: Uuid,
    #[serde(with = "yolo_crypto::serde_non_nil_uuid")]
    pub runtime_epoch: Uuid,
    pub generation: u64,
    pub writer_capability: WriterCapability,
    pub signal: String,
}

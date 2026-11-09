use serde::{Deserialize, Serialize};
use uuid::Uuid;
use yolo_agent_control_contract::NativeControlWitness;
use yolo_agent_input::Prompt;
use yolo_crypto::Sha256Digest;
use yolo_terminal_model::WriterCapability;

/// Origin-admitted intent. The Worker revalidates the source and writer before PTY input.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NativeSubmitRequest {
    #[serde(with = "yolo_crypto::serde_non_nil_uuid")]
    pub terminal_id: Uuid,
    #[serde(with = "yolo_crypto::serde_non_nil_uuid")]
    pub runtime_epoch: Uuid,
    #[serde(with = "yolo_crypto::serde_non_nil_uuid")]
    pub request_id: Uuid,
    pub generation: u64,
    pub writer_capability: WriterCapability,
    pub witness: NativeControlWitness,
    #[serde(with = "yolo_crypto::sha256_digest::compact")]
    pub command_digest: Sha256Digest,
    pub prompt: Prompt,
}

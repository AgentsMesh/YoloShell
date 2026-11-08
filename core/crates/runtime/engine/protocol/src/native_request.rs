use serde::{Deserialize, Serialize};
use uuid::Uuid;
use yolo_agent_control_contract::NativeControlWitness;
use yolo_crypto::Sha256Digest;
use yolo_terminal_model::WriterCapability;

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NativeObservationRequest {
    #[serde(with = "yolo_crypto::serde_non_nil_uuid")]
    pub terminal_id: Uuid,
    #[serde(with = "yolo_crypto::serde_non_nil_uuid")]
    pub runtime_epoch: Uuid,
    #[serde(with = "yolo_crypto::sha256_digest::compact")]
    pub binding_digest: Sha256Digest,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NativeControlLookupRequest {
    #[serde(with = "yolo_crypto::serde_non_nil_uuid")]
    pub terminal_id: Uuid,
    #[serde(with = "yolo_crypto::serde_non_nil_uuid")]
    pub runtime_epoch: Uuid,
    #[serde(with = "yolo_crypto::serde_non_nil_uuid")]
    pub request_id: Uuid,
    #[serde(with = "yolo_crypto::sha256_digest::compact")]
    pub command_digest: Sha256Digest,
}

/// Private trusted-controller request. The digest binds the complete public
/// semantic request selected by the origin; writer credentials authorize this
/// attempt and are not its durable operation identity.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NativeInterruptRequest {
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
}

/// A typed response to a pending provider interaction. The operation is
/// idempotent through `request_id` and resolves at most one pending request.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NativeRespondRequest {
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
    #[serde(with = "yolo_crypto::serde_non_nil_uuid")]
    pub interaction: Uuid,
    pub choice: String,
}

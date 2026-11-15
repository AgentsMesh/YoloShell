use crate::{ControlContractError, validation};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use yolo_agent_contract::AgentKind;
use yolo_crypto::Sha256Digest;

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NativeControlWitnessFields {
    #[serde(with = "yolo_crypto::serde_non_nil_uuid")]
    pub terminal_id: Uuid,
    #[serde(with = "yolo_crypto::serde_non_nil_uuid")]
    pub runtime_epoch: Uuid,
    #[serde(with = "yolo_crypto::serde_non_nil_uuid")]
    pub worker_id: Uuid,
    pub boot_id: String,
    pub kind: AgentKind,
    pub process_pid: u32,
    pub process_start_time: u64,
    #[serde(with = "yolo_crypto::sha256_digest::compact")]
    pub binding_digest: Sha256Digest,
    #[serde(with = "yolo_crypto::sha256_digest::compact")]
    pub authority_digest: Sha256Digest,
}

/// Validated identity metadata from a trusted source. Construction or decoding
/// does not turn a caller-supplied PID/digest into custody or mutation authority.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(
    try_from = "NativeControlWitnessFields",
    into = "NativeControlWitnessFields"
)]
pub struct NativeControlWitness(NativeControlWitnessFields);

impl NativeControlWitness {
    pub fn new(fields: NativeControlWitnessFields) -> Result<Self, ControlContractError> {
        if fields.terminal_id.is_nil() || fields.runtime_epoch.is_nil() || fields.worker_id.is_nil()
        {
            return Err(ControlContractError::InvalidIdentity);
        }
        validation::provider(&fields.kind)?;
        validation::boot(&fields.boot_id)?;
        validation::process(fields.process_pid, fields.process_start_time)?;
        Ok(Self(fields))
    }
    pub fn fields(&self) -> &NativeControlWitnessFields {
        &self.0
    }
}
impl TryFrom<NativeControlWitnessFields> for NativeControlWitness {
    type Error = ControlContractError;
    fn try_from(fields: NativeControlWitnessFields) -> Result<Self, Self::Error> {
        Self::new(fields)
    }
}
impl From<NativeControlWitness> for NativeControlWitnessFields {
    fn from(witness: NativeControlWitness) -> Self {
        witness.0
    }
}

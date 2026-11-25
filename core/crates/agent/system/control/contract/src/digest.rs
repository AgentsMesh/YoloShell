use crate::{ControlContractError, constants::*, validation};
use serde::Serialize;
use yolo_agent_contract::AgentBinding;
use yolo_crypto::Sha256Digest;

/// Fixed typed fields and exact strings form the canonical binding image; no path
/// normalization, filesystem reads or process lookup occurs during digesting.
pub fn binding_digest(binding: &AgentBinding) -> Result<Sha256Digest, ControlContractError> {
    validation::binding(binding)?;
    let value = serde_json::to_value(binding).map_err(|_| ControlContractError::Digest)?;
    Sha256Digest::capture(&(BINDING_DOMAIN, value)).map_err(|_| ControlContractError::Digest)
}

/// Input must be captured by a trusted native source guard. This domain separator
/// prevents confusion with binding hashes; a digest is not authentication by itself.
/// Producers must supply a deterministic typed evidence image.
pub fn authority_digest<T: Serialize + ?Sized>(
    evidence: &T,
) -> Result<Sha256Digest, ControlContractError> {
    Sha256Digest::capture(&(AUTHORITY_DOMAIN, evidence)).map_err(|_| ControlContractError::Digest)
}

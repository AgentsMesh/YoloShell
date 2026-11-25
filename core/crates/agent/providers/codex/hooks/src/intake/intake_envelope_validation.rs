use super::outbox_envelope_validation;
use crate::agent::hooks::{HookEnvelope, HookViolation};
use anyhow::{Result, anyhow};

const REDUCER_REJECTION_CANDIDATES: &[HookViolation] = &[
    HookViolation::ProviderMismatch,
    HookViolation::VersionMismatch,
    HookViolation::UnsupportedEvent,
    HookViolation::InvalidTopology,
    HookViolation::SessionMismatch,
];

pub(super) fn validate(event: &HookEnvelope) -> Result<()> {
    if outbox_envelope_validation::validate(event).is_ok() {
        return Ok(());
    }
    if REDUCER_REJECTION_CANDIDATES
        .iter()
        .copied()
        .any(|violation| outbox_envelope_validation::validate_rejected(event, violation).is_ok())
    {
        return Ok(());
    }
    Err(anyhow!(super::constants::errors::INTAKE_CORRUPT))
}

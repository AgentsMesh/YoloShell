use crate::constants::*;
use std::{error::Error, fmt};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ControlContractError {
    UnsupportedSchema,
    InvalidIdentity,
    InvalidBoot,
    UnsupportedProvider,
    InvalidProcess,
    InvalidBinding,
    BindingMismatch,
    Digest,
    InvalidInteraction,
}
impl fmt::Display for ControlContractError {
    fn fmt(&self, output: &mut fmt::Formatter<'_>) -> fmt::Result {
        output.write_str(match self {
            Self::UnsupportedSchema => SCHEMA_ERROR,
            Self::InvalidIdentity => IDENTITY_ERROR,
            Self::InvalidBoot => BOOT_ERROR,
            Self::UnsupportedProvider => PROVIDER_ERROR,
            Self::InvalidProcess => PROCESS_ERROR,
            Self::InvalidBinding => BINDING_ERROR,
            Self::BindingMismatch => COHERENCE_ERROR,
            Self::Digest => DIGEST_ERROR,
            Self::InvalidInteraction => INTERACTION_ERROR,
        })
    }
}
impl Error for ControlContractError {}

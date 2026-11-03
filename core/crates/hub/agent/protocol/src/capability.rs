use serde::{Deserialize, Serialize};

use crate::ProtocolError;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(tag = "support", rename_all = "snake_case", deny_unknown_fields)]
pub enum Capability {
    Unsupported {},
    Available {},
    Unavailable { reason: UnavailabilityReason },
}

impl Capability {
    pub fn require_available(self) -> Result<(), ProtocolError> {
        match self {
            Self::Unsupported {} => Err(ProtocolError::UnsupportedControl),
            Self::Available {} => Ok(()),
            Self::Unavailable { .. } => Err(ProtocolError::UnavailableControl),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum UnavailabilityReason {
    NotRunning,
    NoPendingInteraction,
    InputUnavailable,
    AdapterUnavailable,
    StateUnknown,
}

/// Adapter capability is not a grant of permission to the observing client.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AgentCapabilities {
    pub interrupt: Capability,
    pub submit_prompt: Capability,
    pub respond_interaction: Capability,
}

impl AgentCapabilities {
    pub const UNSUPPORTED: Self = Self {
        interrupt: Capability::Unsupported {},
        submit_prompt: Capability::Unsupported {},
        respond_interaction: Capability::Unsupported {},
    };
}

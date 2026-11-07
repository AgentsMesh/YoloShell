use super::constants::{
    FIRST_SEQUENCE, INITIAL_GENERATION, ZERO_PROCESS_ID, ZERO_PROCESS_START_TIME, ZERO_SEQUENCE,
};
use super::violation::HookViolation;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProviderVersion {
    pub major: u16,
    pub minor: u16,
    pub patch: u16,
}

impl ProviderVersion {
    pub const fn new(major: u16, minor: u16, patch: u16) -> Self {
        Self {
            major,
            minor,
            patch,
        }
    }

    pub const fn at_least(self, required: Self) -> bool {
        self.major > required.major
            || (self.major == required.major
                && (self.minor > required.minor
                    || (self.minor == required.minor && self.patch >= required.patch)))
    }
}

#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HookProviderKind {
    Codex,
    Claude,
    Other(String),
}

impl HookProviderKind {
    pub const fn codex_name() -> &'static str {
        super::constants::provider_names::CODEX
    }

    pub const fn claude_name() -> &'static str {
        super::constants::provider_names::CLAUDE
    }

    pub fn as_str(&self) -> &str {
        match self {
            Self::Codex => Self::codex_name(),
            Self::Claude => Self::claude_name(),
            Self::Other(name) => name,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
pub struct EventSequence(pub u64);

impl EventSequence {
    pub const ZERO: Self = Self(ZERO_SEQUENCE);
    pub const FIRST: Self = Self(FIRST_SEQUENCE);

    pub fn next(self) -> Option<Self> {
        match self.0.checked_add(FIRST_SEQUENCE) {
            Some(value) => Some(Self(value)),
            None => None,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
pub struct Generation(pub u64);

impl Generation {
    pub const INITIAL: Self = Self(INITIAL_GENERATION);
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProcessIdentity {
    pub pid: u32,
    pub start_time: u64,
}

impl ProcessIdentity {
    pub const fn new(pid: u32, start_time: u64) -> Self {
        Self { pid, start_time }
    }

    pub const fn is_valid(self) -> bool {
        self.pid != ZERO_PROCESS_ID && self.start_time != ZERO_PROCESS_START_TIME
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HookIdentity {
    #[serde(with = "yolo_crypto::serde_uuid")]
    pub terminal_id: Uuid,
    #[serde(with = "yolo_crypto::serde_uuid")]
    pub runtime_epoch: Uuid,
    #[serde(with = "yolo_crypto::serde_uuid")]
    pub invocation_id: Uuid,
    #[serde(with = "yolo_crypto::serde_uuid")]
    pub hook_token: Uuid,
    pub process: ProcessIdentity,
    pub generation: Generation,
}

pub type ExpectedIdentity = HookIdentity;

impl HookIdentity {
    pub fn validate(&self) -> Result<(), HookViolation> {
        let identifiers = [
            self.terminal_id,
            self.runtime_epoch,
            self.invocation_id,
            self.hook_token,
        ];
        if identifiers.iter().any(Uuid::is_nil)
            || !self.process.is_valid()
            || self.generation.0 == ZERO_SEQUENCE
        {
            return Err(HookViolation::InvalidIdentity);
        }
        Ok(())
    }
}

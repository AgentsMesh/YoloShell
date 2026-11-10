use crate::paths::identity::{DurableFileVersion, DurableObjectIdentity};
use crate::sha256_digest::Sha256Digest;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(tag = "state", rename_all = "snake_case", deny_unknown_fields)]
pub(super) enum PreparationAuthority {
    Active(ActivePreparationAuthority),
    Settled(SettledPreparationAuthority),
    Retired(RetiredPreparationAuthority),
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct ActivePreparationAuthority {
    pub(super) schema_version: u16,
    #[serde(with = "crate::serde_non_nil_uuid")]
    pub(super) transaction_id: Uuid,
    #[serde(with = "crate::serde_non_nil_uuid")]
    pub(super) terminal_id: Uuid,
    pub(super) session_directory: DurableObjectIdentity,
    pub(super) startup_checkpoint: ParticipantPreparation,
    pub(super) claude_custody: ParticipantPreparation,
    pub(super) claude_generation: ParticipantPreparation,
    pub(super) pty_custody: ParticipantPreparation,
    pub(super) runtime: ParticipantPreparation,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct RetiredPreparationAuthority {
    pub(super) schema_version: u16,
    #[serde(with = "crate::serde_non_nil_uuid")]
    pub(super) terminal_id: Uuid,
    pub(super) session_directory: DurableObjectIdentity,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct SettledPreparationAuthority {
    pub(super) schema_version: u16,
    #[serde(with = "crate::serde_non_nil_uuid")]
    pub(super) transaction_id: Uuid,
    #[serde(with = "crate::serde_non_nil_uuid")]
    pub(super) terminal_id: Uuid,
    pub(super) session_directory: DurableObjectIdentity,
    pub(super) active_version: DurableFileVersion,
    pub(super) startup_checkpoint: SettledParticipant,
    pub(super) claude_custody: SettledParticipant,
    pub(super) claude_generation: SettledParticipant,
    pub(super) pty_custody: SettledParticipant,
    pub(super) runtime: SettledParticipant,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub(super) enum ParticipantPreparation {
    Absent,
    Replace {
        source_version: DurableFileVersion,
        source_sha256: Sha256Digest,
        target_sha256: Sha256Digest,
    },
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub(super) enum SettledParticipant {
    Absent,
    Installed {
        version: DurableFileVersion,
        sha256: Sha256Digest,
    },
}

impl PreparationAuthority {
    pub(super) fn terminal_id(&self) -> Uuid {
        match self {
            Self::Active(value) => value.terminal_id,
            Self::Settled(value) => value.terminal_id,
            Self::Retired(value) => value.terminal_id,
        }
    }

    pub(super) fn session_directory(&self) -> DurableObjectIdentity {
        match self {
            Self::Active(value) => value.session_directory,
            Self::Settled(value) => value.session_directory,
            Self::Retired(value) => value.session_directory,
        }
    }
}

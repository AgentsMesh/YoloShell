use crate::paths::identity::{DurableFileVersion, DurableObjectIdentity};
use crate::sha256_digest::Sha256Digest;
#[cfg(any(test, feature = "adversarial-test-hooks"))]
use anyhow::Result;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum RebootMigrationPhase {
    Prepared,
    PrerequisitesInstalled,
    RuntimeInstalled,
    Verified,
}

impl RebootMigrationPhase {
    pub(super) const fn successor(self) -> Option<Self> {
        match self {
            Self::Prepared => Some(Self::PrerequisitesInstalled),
            Self::PrerequisitesInstalled => Some(Self::RuntimeInstalled),
            Self::RuntimeInstalled => Some(Self::Verified),
            Self::Verified => None,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct RebootMigrationIntent {
    pub(crate) schema_version: u16,
    #[serde(with = "crate::serde_non_nil_uuid")]
    pub(crate) transaction_id: Uuid,
    #[serde(with = "crate::serde_non_nil_uuid")]
    pub(crate) terminal_id: Uuid,
    #[serde(with = "crate::serde_non_nil_uuid")]
    pub(crate) runtime_epoch: Uuid,
    #[serde(with = "crate::serde_non_nil_uuid")]
    pub(crate) worker_id: Uuid,
    pub(crate) controller_generation: u64,
    pub(crate) source_boot_id: String,
    pub(crate) target_boot_id: String,
    pub(crate) session_directory: DurableObjectIdentity,
    pub(crate) preparation_authority_version: DurableFileVersion,
    pub(crate) phase: RebootMigrationPhase,
    pub startup_checkpoint:
        OptionalParticipant<yolo_startup_checkpoint_contract::StartupCheckpointEvidence>,
    pub(crate) claude_custody: OptionalParticipant<crate::claude_custody::CustodyLedger>,
    pub(crate) claude_generation:
        OptionalParticipant<crate::agent::claude::generation::ClaudeInvocationAuthorities>,
    pub(crate) pty_custody: OptionalParticipant<crate::pty_custody::CustodyDurableRecord>,
    pub(crate) runtime: RequiredParticipant<crate::model::RuntimeRecord>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
#[serde(bound(
    serialize = "T: Serialize",
    deserialize = "T: serde::de::DeserializeOwned"
))]
pub(crate) enum OptionalParticipant<T> {
    Absent,
    Replace(RequiredParticipant<T>),
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
#[serde(bound(
    serialize = "T: Serialize",
    deserialize = "T: serde::de::DeserializeOwned"
))]
pub(crate) struct RequiredParticipant<T> {
    pub(crate) source_version: DurableFileVersion,
    pub(crate) source_sha256: Sha256Digest,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) source_object_sha256: Option<Sha256Digest>,
    pub(crate) target_version: DurableFileVersion,
    pub(crate) prepared_receipt_version: DurableFileVersion,
    pub(crate) target_sha256: Sha256Digest,
    pub(crate) target_payload: T,
}

impl<T> RequiredParticipant<T>
where
    T: Clone + Serialize,
{
    #[cfg(any(test, feature = "adversarial-test-hooks"))]
    pub(crate) fn new(
        source_version: DurableFileVersion,
        target_version: DurableFileVersion,
        prepared_receipt_version: DurableFileVersion,
        source: &T,
        target: &T,
    ) -> Result<Self> {
        Ok(Self {
            source_version,
            source_sha256: Sha256Digest::capture(source)?,
            source_object_sha256: None,
            target_version,
            prepared_receipt_version,
            target_sha256: Sha256Digest::capture(target)?,
            target_payload: target.clone(),
        })
    }
}

use crate::JournalScope;
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use yolo_paths::identity::{FileIdentity, ObjectIdentity};

/// Persist outside this directory with the trusted runtime initialization record.
/// A valid anchor identifies a journal; it grants no execution reservation.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct JournalAnchor {
    #[serde(with = "yolo_crypto::serde_non_nil_uuid")]
    pub(crate) journal_id: Uuid,
    pub(crate) scope: JournalScope,
    pub(crate) directory: ObjectIdentity,
    pub(crate) header_identity: FileIdentity,
}
impl JournalAnchor {
    pub fn journal_id(self) -> Uuid {
        self.journal_id
    }
    pub fn scope(self) -> JournalScope {
        self.scope
    }
}

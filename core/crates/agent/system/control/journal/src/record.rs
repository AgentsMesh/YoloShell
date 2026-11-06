use crate::{JournalAnchor, JournalError, JournalScope, Receipt, constants::*};
use anyhow::{Result, ensure};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use uuid::Uuid;
use yolo_crypto::Sha256Digest;
use yolo_paths::identity::{FileIdentity, ObjectIdentity};

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Header {
    pub schema: u16,
    #[serde(with = "yolo_crypto::serde_non_nil_uuid")]
    pub journal_id: Uuid,
    pub scope: JournalScope,
    pub directory: ObjectIdentity,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Record {
    pub schema: u16,
    pub anchor: JournalAnchor,
    pub entries: Vec<Entry>,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Entry {
    #[serde(with = "yolo_crypto::serde_non_nil_uuid")]
    pub request_id: Uuid,
    #[serde(with = "yolo_crypto::sha256_digest::compact")]
    pub digest: Sha256Digest,
    pub receipt: Receipt,
}
impl Header {
    pub fn new(scope: JournalScope, directory: ObjectIdentity) -> Self {
        Self {
            schema: SCHEMA_VERSION,
            journal_id: Uuid::new_v4(),
            scope,
            directory,
        }
    }
    pub fn anchor(self, header_identity: FileIdentity) -> JournalAnchor {
        JournalAnchor {
            journal_id: self.journal_id,
            scope: self.scope,
            directory: self.directory,
            header_identity,
        }
    }
    pub fn validate(self, expected: JournalAnchor) -> Result<()> {
        ensure!(self.schema == SCHEMA_VERSION, INVALID_RECORD);
        ensure!(
            self.journal_id == expected.journal_id
                && !self.journal_id.is_nil()
                && self.scope == expected.scope
                && self.directory == expected.directory,
            WRONG_ANCHOR
        );
        Ok(())
    }
}
impl Record {
    pub fn empty(anchor: JournalAnchor) -> Self {
        Self {
            schema: SCHEMA_VERSION,
            anchor,
            entries: Vec::new(),
        }
    }
    pub fn validate(&self, expected: JournalAnchor) -> Result<()> {
        ensure!(self.schema == SCHEMA_VERSION, INVALID_RECORD);
        ensure!(
            self.anchor == expected && !self.anchor.journal_id.is_nil(),
            WRONG_ANCHOR
        );
        ensure!(self.entries.len() <= MAX_RECEIPTS, CAPACITY);
        let mut ids = BTreeSet::new();
        for entry in &self.entries {
            ensure!(
                !entry.request_id.is_nil() && ids.insert(entry.request_id),
                INVALID_RECORD
            );
        }
        Ok(())
    }
    pub fn lookup(
        &self,
        request_id: Uuid,
        digest: Sha256Digest,
    ) -> Result<Option<Receipt>, JournalError> {
        match self
            .entries
            .iter()
            .find(|entry| entry.request_id == request_id)
        {
            Some(entry) if entry.digest == digest => Ok(Some(entry.receipt)),
            Some(_) => Err(JournalError::Conflict),
            None => Ok(None),
        }
    }
}

use crate::{
    JournalAnchor, JournalError,
    constants::*,
    record::{Header, Record},
};
use anyhow::{Result, ensure};
use std::{
    path::PathBuf,
    sync::atomic::{AtomicBool, Ordering},
};
use uuid::Uuid;
use yolo_paths::{
    coordination::CoordinationLock, identity::FileIdentity, namespace_lease::NamespaceLease,
};

pub struct Journal {
    pub(crate) header_path: PathBuf,
    pub(crate) record_path: PathBuf,
    pub(crate) namespace: NamespaceLease,
    pub(crate) lock: CoordinationLock,
    pub(crate) header: Header,
    pub(crate) header_identity: FileIdentity,
    pub(crate) record: Record,
    pub(crate) record_identity: FileIdentity,
    pub(crate) owner: Uuid,
    pub(crate) poisoned: AtomicBool,
}
impl Journal {
    pub fn anchor(&self) -> JournalAnchor {
        self.header.anchor(self.header_identity)
    }

    pub fn verify(&self) -> Result<(), JournalError> {
        self.retain_failure(self.verify_inner())
    }

    pub(crate) fn verify_inner(&self) -> Result<(), JournalError> {
        if self.poisoned.load(Ordering::Acquire) {
            return Err(JournalError::Poisoned);
        }
        let verified = (|| -> Result<()> {
            self.verify_authority()?;
            let (record, identity): (Record, _) =
                self.namespace.read_json_with_identity(&self.record_path)?;
            ensure!(
                identity == self.record_identity && record == self.record,
                REPLACED_RECORD
            );
            record.validate(self.anchor())?;
            self.verify_authority()
        })();
        verified.map_err(|_| JournalError::Unavailable)
    }

    pub(crate) fn verify_authority(&self) -> Result<()> {
        ensure!(!self.poisoned.load(Ordering::Acquire), POISONED);
        self.lock.verify()?;
        self.namespace.verify()?;
        let (header, identity): (Header, _) =
            self.namespace.read_json_with_identity(&self.header_path)?;
        ensure!(
            identity == self.header_identity && header == self.header,
            REPLACED_HEADER
        );
        header.validate(self.anchor())?;
        self.namespace.verify()
    }

    pub(crate) fn commit(&mut self, next: Record) -> Result<(), JournalError> {
        self.verify_inner()?;
        next.validate(self.anchor())
            .map_err(|_| JournalError::Unavailable)?;
        let identity = self
            .namespace
            .write_json_expected_with_commit_fence(
                &self.record_path,
                Some(self.record_identity),
                &next,
                || self.verify_authority(),
            )
            .map_err(|_| JournalError::OutcomeUnknown)?;
        self.record = next;
        self.record_identity = identity;
        self.verify_inner()
            .map_err(|_| JournalError::OutcomeUnknown)
    }

    pub(crate) fn retain_failure<T>(
        &self,
        result: Result<T, JournalError>,
    ) -> Result<T, JournalError> {
        if result.as_ref().err().is_some_and(|error| error.poisons()) {
            self.poisoned.store(true, Ordering::Release);
        }
        result
    }
}

use crate::{
    constants::*,
    record::{Document, PendingFinalization},
};
use anyhow::{Result, ensure};
use std::{
    path::PathBuf,
    sync::atomic::{AtomicBool, Ordering},
};
use yolo_paths::{
    coordination::CoordinationLock, identity::FileIdentity, namespace_lease::NamespaceLease,
};

/// Single owner of creation intents and receipts. Failed operations poison this instance;
/// recover() only admits the retained state or an exactly observed own commit.
pub struct CreationJournal {
    pub(crate) namespace: NamespaceLease,
    pub(crate) lock: CoordinationLock,
    pub(crate) path: PathBuf,
    pub(crate) identity: FileIdentity,
    pub(crate) document: Document,
    pub(crate) lost: AtomicBool,
    pub(crate) recovery: Option<(Document, FileIdentity)>,
}

impl CreationJournal {
    pub fn pending(&self) -> Result<Vec<PendingFinalization>> {
        self.verify()?;
        Ok(self.document.pending.clone())
    }

    pub fn verify(&self) -> Result<()> {
        self.retain_failure(self.check_current())
    }

    pub fn retain_confirmed(&mut self, record: PendingFinalization) -> Result<bool> {
        let result = (|| {
            self.check_current()?;
            record.validate()?;
            if let Some(existing) = self
                .document
                .pending
                .iter()
                .find(|other| record.overlaps(other))
            {
                ensure!(*existing == record, CONFLICT);
                return Ok(false);
            }
            let mut next = self.document.clone();
            if let Some(index) = next
                .intents
                .iter()
                .position(|intent| intent.overlaps_request(&record.request))
            {
                ensure!(
                    next.intents[index].origin == record.origin
                        && next.intents[index].request == record.request,
                    CONFLICT
                );
                // Publish the receipt and retire the exact intent in one fenced commit.
                next.intents.remove(index);
            }
            ensure!(
                next.pending.len() + next.intents.len() < MAX_PENDING_CREATIONS,
                CAPACITY
            );
            next.pending.push(record);
            self.install(next)?;
            Ok(true)
        })();
        self.retain_failure(result)
    }

    /// Call only after a correlated successful Finalize response for this exact receipt.
    pub fn acknowledge(&mut self, exact: &PendingFinalization) -> Result<bool> {
        let result = (|| {
            self.check_current()?;
            exact.validate()?;
            let Some(index) = self
                .document
                .pending
                .iter()
                .position(|other| exact.overlaps(other))
            else {
                return Ok(false);
            };
            ensure!(self.document.pending[index] == *exact, CONFLICT);
            let mut next = self.document.clone();
            next.pending.remove(index);
            self.install(next)?;
            Ok(true)
        })();
        self.retain_failure(result)
    }

    pub(crate) fn check_current(&self) -> Result<()> {
        ensure!(!self.lost.load(Ordering::Acquire), LOST);
        self.lock.verify()?;
        self.namespace.verify()?;
        let (document, identity): (Document, _) =
            self.namespace.read_json_with_identity(&self.path)?;
        ensure!(
            identity == self.identity && document == self.document,
            CHANGED
        );
        document.validate(self.document.entry_hub, self.document.directory)?;
        self.lock.verify()?;
        self.namespace.verify()
    }

    pub(crate) fn retain_failure<T>(&self, result: Result<T>) -> Result<T> {
        if result.is_err() {
            self.lost.store(true, Ordering::Release);
        }
        result
    }
}

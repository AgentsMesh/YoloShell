use crate::{StoreError, constants::*, record::Record};
use anyhow::{Result, ensure};
use std::{
    path::PathBuf,
    sync::atomic::{AtomicBool, Ordering},
};
use yolo_paths::{
    coordination::CoordinationLock, identity::FileIdentity, namespace_lease::NamespaceLease,
};
use yolo_presentation_protocol::{LayoutAddress, SharedLayoutDocument, LayoutUpdate, ProtocolError};

pub struct LayoutStore {
    pub(crate) namespace: NamespaceLease,
    pub(crate) lock: CoordinationLock,
    pub(crate) path: PathBuf,
    pub(crate) identity: FileIdentity,
    pub(crate) record: Record,
    pub(crate) lost: AtomicBool,
}

impl LayoutStore {
    pub fn list(&self) -> Result<Vec<SharedLayoutDocument>, StoreError> {
        self.check()?;
        let mut documents = self.record.documents.clone();
        documents.sort_by_key(|document| document.address);
        Ok(documents)
    }

    pub fn get(&self, address: LayoutAddress) -> Result<Option<SharedLayoutDocument>, StoreError> {
        self.admit(address)?;
        self.check()?;
        Ok(self
            .record
            .documents
            .iter()
            .find(|document| document.address == address)
            .cloned())
    }

    pub fn update(&mut self, update: LayoutUpdate) -> Result<SharedLayoutDocument, StoreError> {
        self.admit(update.address)?;
        update.validate()?;
        self.check()?;
        let position = self
            .record
            .documents
            .iter()
            .position(|document| document.address == update.address);
        let next = update.apply(position.map(|index| &self.record.documents[index]))?;
        if position.is_none() && self.record.documents.len() >= MAX_DOCUMENTS {
            return Err(ProtocolError::Limit.into());
        }
        let mut record = self.record.clone();
        if let Some(index) = position {
            record.documents[index] = next.clone();
        } else {
            record.documents.push(next.clone());
        }
        record.validate(self.record.hub_id)?;
        let result = self.namespace.write_json_expected_with_commit_fence(
            &self.path,
            Some(self.identity),
            &record,
            || {
                self.lock.verify()?;
                self.namespace.verify()
            },
        );
        let identity = self.retain(result)?;
        self.record = record;
        self.identity = identity;
        Ok(next)
    }

    fn admit(&self, address: LayoutAddress) -> Result<(), StoreError> {
        if address.hub_id != self.record.hub_id {
            return Err(ProtocolError::AddressMismatch.into());
        }
        if address.layout_id.is_nil() {
            return Err(ProtocolError::InvalidId.into());
        }
        Ok(())
    }

    pub(crate) fn check(&self) -> Result<(), StoreError> {
        self.retain((|| {
            ensure!(!self.lost.load(Ordering::Acquire), LOST);
            self.namespace.verify()?;
            self.lock.verify()?;
            let (record, identity): (Record, _) =
                self.namespace.read_json_with_identity(&self.path)?;
            ensure!(record == self.record && identity == self.identity, CHANGED);
            self.lock.verify()
        })())
    }

    fn retain<T>(&self, result: Result<T>) -> Result<T, StoreError> {
        if result.is_err() {
            self.lost.store(true, Ordering::Release);
        }
        result.map_err(StoreError::Storage)
    }
}

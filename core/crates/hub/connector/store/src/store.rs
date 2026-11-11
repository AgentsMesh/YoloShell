use crate::{
    Seed, StoredState,
    constants::*,
    record::{Index, Record},
};
use anyhow::{Result, ensure};
use std::{
    collections::BTreeMap,
    path::PathBuf,
    sync::atomic::{AtomicBool, Ordering},
};
use yolo_hub_delegation::PeerIdentity;
use yolo_hub_identity::ConnectorId;
use yolo_paths::{
    coordination::CoordinationLock,
    identity::{DurableObjectIdentity, FileIdentity},
    namespace_lease::NamespaceLease,
};

/// A singleton durable store. Every failed operation poisons this instance;
/// callers must reopen it before making another observation or publication.
/// Business-level duplicate/revision checks belong to the enclosing registry.
pub struct Store<T: StoredState> {
    pub(crate) identity: PeerIdentity,
    pub(crate) directory: DurableObjectIdentity,
    pub(crate) members_directory: DurableObjectIdentity,
    pub(crate) path: PathBuf,
    pub(crate) members_path: PathBuf,
    pub(crate) namespace: NamespaceLease,
    pub(crate) lock: CoordinationLock,
    pub(crate) index: Index<T>,
    pub(crate) index_identity: FileIdentity,
    pub(crate) member_identities: BTreeMap<ConnectorId, FileIdentity>,
    pub(crate) lost: AtomicBool,
}

impl<T: StoredState> Store<T> {
    pub fn entries(&self) -> Result<Vec<(Seed, T)>> {
        self.retain_failure((|| {
            self.check_index()?;
            self.check_inventory()?;
            let records = self
                .index
                .members
                .iter()
                .map(|member| {
                    let record = self.read_member(member.id())?;
                    Ok((record.seed, record.state))
                })
                .collect::<Result<_>>()?;
            self.check_index()?;
            Ok(records)
        })())
    }
    pub fn get(&self, id: ConnectorId) -> Result<Option<(Seed, T)>> {
        self.retain_failure((|| {
            self.check_index()?;
            self.check_inventory()?;
            let record = self
                .index
                .member(id)
                .map(|_| self.read_member(id))
                .transpose()?;
            self.check_index()?;
            Ok(record.map(|record| (record.seed, record.state)))
        })())
    }
    pub fn insert(&mut self, seed: Seed, state: T) -> Result<()> {
        let result = (|| {
            let id = seed.connector_id;
            self.prepare(Record { seed, state })?;
            self.stage_member(id)?;
            self.finish(id)
        })();
        self.retain_failure(result)
    }
    pub fn update(&mut self, id: ConnectorId, state: T) -> Result<()> {
        let result = (|| {
            self.check_index()?;
            self.check_inventory()?;
            let previous = self.read_member(id)?;
            let record = Record {
                seed: previous.seed,
                state,
            };
            record.validate(self.identity)?;
            let expected = self
                .member_identities
                .get(&id)
                .copied()
                .ok_or_else(|| anyhow::anyhow!(MEMBER_MISSING))?;
            let identity = self.namespace.write_json_expected_with_commit_fence(
                &self.member_path(id),
                Some(expected),
                &record,
                || self.check_index(),
            )?;
            self.member_identities.insert(id, identity);
            self.check_index()?;
            ensure!(self.read_member(id)? == record, STATE_MISMATCH);
            self.check_inventory()?;
            Ok(())
        })();
        self.retain_failure(result)
    }
    pub fn verify(&self) -> Result<()> {
        self.retain_failure((|| {
            self.check_index()?;
            self.check_inventory()?;
            for member in &self.index.members {
                self.read_member(member.id())?;
            }
            self.check_index()
        })())
    }
    pub(crate) fn verify_namespace(&self) -> Result<()> {
        ensure!(!self.lost.load(Ordering::Acquire), LOST);
        self.lock.verify()?;
        self.namespace.verify()
    }
    pub(crate) fn retain_failure<U>(&self, result: Result<U>) -> Result<U> {
        if result.is_err() {
            self.lost.store(true, Ordering::Release);
        }
        result
    }
}

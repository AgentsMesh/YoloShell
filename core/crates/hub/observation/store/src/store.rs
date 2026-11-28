use crate::{Retirement, constants::*, index::Index, record::Record};
use anyhow::{Result, ensure};
use std::{
    collections::BTreeMap,
    path::PathBuf,
    sync::atomic::{AtomicBool, Ordering},
};
use yolo_hub_federation_directory::ObservationHistory;
use yolo_hub_identity::HubId;
use yolo_hub_routing::RoutingLimits;
use yolo_paths::{
    coordination::CoordinationLock, identity::FileIdentity, namespace_lease::NamespaceLease,
};

/// A generation-owned cache writer. It has no process, delegation or client authority.
pub struct ObservationStore {
    pub(crate) index: Index,
    pub(crate) records: BTreeMap<HubId, (Record, FileIdentity)>,
    pub(crate) root: PathBuf,
    pub(crate) path: PathBuf,
    pub(crate) namespace: NamespaceLease,
    pub(crate) lock: CoordinationLock,
    pub(crate) index_file: FileIdentity,
    pub(crate) limits: RoutingLimits,
    pub(crate) lost: AtomicBool,
    pub(crate) retirement: Retirement,
}
impl ObservationStore {
    pub fn history(&self) -> Result<ObservationHistory> {
        self.retain(self.read())
    }
    pub(crate) fn record_path(&self, origin: HubId) -> PathBuf {
        self.root
            .join(origin.to_string())
            .with_extension(ORIGIN_EXTENSION)
    }
    pub(crate) fn read(&self) -> Result<ObservationHistory> {
        self.check_index()?;
        let mut catalogues = Vec::new();
        for origin in &self.index.origins {
            self.retirement.verify()?;
            let (expected, version) = self
                .records
                .get(origin)
                .ok_or_else(|| anyhow::anyhow!(CHANGED))?;
            let (record, file): (Record, FileIdentity) = self
                .namespace
                .read_json_with_identity(&self.record_path(*origin))?;
            ensure!(
                file == *version && record == *expected && record.catalogue.signer.hub() == *origin,
                CHANGED
            );
            record.history(self.index.directory, self.index.entry, self.limits)?;
            catalogues.push(record.catalogue);
        }
        self.authority()?;
        ObservationHistory::new(self.index.entry, catalogues, self.limits)
    }
    pub(crate) fn check_index(&self) -> Result<()> {
        self.authority()?;
        let (index, file): (Index, FileIdentity) =
            self.namespace.read_json_with_identity(&self.path)?;
        ensure!(file == self.index_file && index == self.index, CHANGED);
        index.validate(self.index.directory, self.index.entry, self.limits)?;
        self.authority()
    }
    pub(crate) fn authority(&self) -> Result<()> {
        self.retirement.verify()?;
        ensure!(!self.lost.load(Ordering::Acquire), LOST);
        self.namespace.verify()?;
        self.lock.verify()
    }
    pub(crate) fn retain<T>(&self, result: Result<T>) -> Result<T> {
        if result.is_err() {
            self.lost.store(true, Ordering::Release);
        }
        result
    }
}

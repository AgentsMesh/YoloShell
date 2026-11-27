//! Durable store mechanics: open, check and commit.

use anyhow::{Context, Result, ensure};
use std::collections::BTreeSet;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use yolo_hub_agent_run_protocol::AgentRunAddress;
use yolo_hub_identity::HubId;
use yolo_paths::{
    coordination::CoordinationLock, directory::PrivateDirectory, identity::FileIdentity,
    namespace_lease::NamespaceLease,
};

use crate::StoreError;
use crate::record::{RunRecord, StoredRun};

const RUN_FILE: &str = "agent-runs.json";
const LOCK_FILE: &str = ".agent-runs.lock";
const LOCK_BYTES: u64 = 64;
const NAMESPACE: &str = "hub-agent-runs";
const BUSY: &str = "agent run store is already open";
const LOST: &str = "agent run store lost its durable identity";
const CHANGED: &str = "agent run store changed on disk";

pub struct AgentRunStore {
    pub(super) namespace: NamespaceLease,
    pub(super) lock: CoordinationLock,
    pub(super) path: PathBuf,
    pub(super) identity: FileIdentity,
    pub(super) record: RunRecord,
    pub(super) active_starts: BTreeSet<AgentRunAddress>,
    pub(super) lost: AtomicBool,
}

impl AgentRunStore {
    pub fn open(directory: PrivateDirectory, hub_id: HubId) -> Result<Self, StoreError> {
        directory.verify_path()?;
        let path = directory.path().join(RUN_FILE);
        let lock = CoordinationLock::try_acquire(&directory.path().join(LOCK_FILE), LOCK_BYTES)?
            .context(BUSY)?;
        directory.verify_path()?;
        let namespace = NamespaceLease::from_directories(vec![(NAMESPACE, directory)])?;
        let (record, identity) = if namespace.file_is_missing(&path)? {
            let record = RunRecord::empty(hub_id);
            let identity =
                namespace.write_json_expected_with_commit_fence(&path, None, &record, || {
                    lock.verify()
                })?;
            (record, identity)
        } else {
            namespace.read_json_with_identity::<RunRecord>(&path)?
        };
        record.validate(hub_id)?;
        let mut store = Self {
            namespace,
            lock,
            path,
            identity,
            record,
            active_starts: BTreeSet::new(),
            lost: AtomicBool::new(false),
        };
        store.check()?;
        store.reconcile_starting()?;
        Ok(store)
    }

    pub(super) fn commit(&mut self, next: StoredRun) -> Result<(), StoreError> {
        let mut record = self.record.clone();
        match record
            .runs
            .iter()
            .position(|run| run.address == next.address)
        {
            Some(index) => record.runs[index] = next,
            None => record.runs.push(next),
        }
        self.commit_record(record)
    }

    pub(super) fn commit_record(&mut self, record: RunRecord) -> Result<(), StoreError> {
        record.validate(record.hub_id)?;
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
        Ok(())
    }

    pub(super) fn check(&self) -> Result<(), StoreError> {
        self.retain((|| {
            ensure!(!self.lost.load(Ordering::Acquire), LOST);
            self.namespace.verify()?;
            self.lock.verify()?;
            let (record, identity): (RunRecord, _) =
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

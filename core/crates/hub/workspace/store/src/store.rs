//! Workspace store mechanics: open, check, commit and query.

use anyhow::{Context, Result, ensure};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use yolo_hub_identity::HubId;
use yolo_hub_workspace_protocol::{
    ListWorkspacesRequest, WorkspaceAddress, WorkspaceSummary,
};
use yolo_paths::{
    coordination::CoordinationLock, directory::PrivateDirectory, identity::FileIdentity,
    namespace_lease::NamespaceLease,
};

use crate::StoreError;
use crate::record::{StoredWorkspace, WorkspaceRecord, summary};

const WORKSPACE_FILE: &str = "workspaces.json";
const LOCK_FILE: &str = ".workspaces.lock";
const LOCK_BYTES: u64 = 64;
const NAMESPACE: &str = "hub-workspaces";
const BUSY: &str = "workspace store is already open";
const LOST: &str = "workspace store lost its durable identity";
const CHANGED: &str = "workspace store changed on disk";

pub struct WorkspaceStore {
    pub(super) namespace: NamespaceLease,
    pub(super) lock: CoordinationLock,
    pub(super) path: PathBuf,
    pub(super) identity: FileIdentity,
    pub(super) record: WorkspaceRecord,
    pub(super) pending: crate::reservation::PendingCreations,
    pub(super) lost: AtomicBool,
}

impl WorkspaceStore {
    pub fn open(directory: PrivateDirectory, hub_id: HubId) -> Result<Self, StoreError> {
        directory.verify_path()?;
        let path = directory.path().join(WORKSPACE_FILE);
        let lock = CoordinationLock::try_acquire(&directory.path().join(LOCK_FILE), LOCK_BYTES)?
            .context(BUSY)?;
        directory.verify_path()?;
        let namespace = NamespaceLease::from_directories(vec![(NAMESPACE, directory)])?;
        let (record, identity) = if namespace.file_is_missing(&path)? {
            let record = WorkspaceRecord::empty(hub_id);
            let identity =
                namespace.write_json_expected_with_commit_fence(&path, None, &record, || {
                    lock.verify()
                })?;
            (record, identity)
        } else {
            namespace.read_json_with_identity::<WorkspaceRecord>(&path)?
        };
        record.validate(hub_id)?;
        let store = Self {
            namespace,
            lock,
            path,
            identity,
            record,
            pending: Default::default(),
            lost: AtomicBool::new(false),
        };
        store.check()?;
        Ok(store)
    }

    pub fn get(&self, address: WorkspaceAddress) -> Result<Option<WorkspaceSummary>, StoreError> {
        self.check()?;
        Ok(self.record.find(address).map(summary))
    }

    pub fn find_name(&self, name: &str) -> Result<Option<WorkspaceSummary>, StoreError> {
        self.check()?;
        Ok(self.record.find_name(name).map(summary))
    }

    pub fn list(
        &self,
        request: &ListWorkspacesRequest,
    ) -> Result<(Vec<WorkspaceSummary>, Option<WorkspaceAddress>), StoreError> {
        self.check()?;
        let after = request.fields().after;
        let limit = request.fields().limit as usize;
        let mut rows: Vec<WorkspaceSummary> = self
            .record
            .workspaces
            .iter()
            .filter(|workspace| after.is_none_or(|after| workspace.address > after))
            .map(summary)
            .collect();
        rows.sort_by_key(|workspace| workspace.address);
        let more = rows.len() > limit;
        rows.truncate(limit);
        let next = more.then(|| rows.last().map(|row| row.address)).flatten();
        Ok((rows, next))
    }

    pub(super) fn commit(&mut self, next: StoredWorkspace) -> Result<(), StoreError> {
        let mut record = self.record.clone();
        match record
            .workspaces
            .iter()
            .position(|workspace| workspace.address == next.address)
        {
            Some(index) => record.workspaces[index] = next,
            None => record.workspaces.push(next),
        }
        self.commit_record(record)
    }

    /// The sole durable write path. Callers mutate a clone and commit it; the
    /// fence, validation, identity swap and failure latch stay in one owner.
    pub(super) fn commit_record(&mut self, record: WorkspaceRecord) -> Result<(), StoreError> {
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
            let (record, identity): (WorkspaceRecord, _) =
                self.namespace.read_json_with_identity(&self.path)?;
            ensure!(record == self.record && identity == self.identity, CHANGED);
            self.lock.verify()
        })())
    }

    pub(super) fn retain<T>(&self, result: Result<T>) -> Result<T, StoreError> {
        if result.is_err() {
            self.lost.store(true, Ordering::Release);
        }
        result.map_err(StoreError::Storage)
    }
}

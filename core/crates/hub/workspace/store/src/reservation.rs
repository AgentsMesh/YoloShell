//! In-memory protection for workspace creation that has not bound a member
//! yet. It is not durable: a restart drops the reservation with the launch.

use std::collections::BTreeMap;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use yolo_hub_terminal_protocol::PublicError;
use yolo_hub_workspace_protocol::WorkspaceAddress;

use crate::StoreError;
use crate::store::WorkspaceStore;

#[must_use = "the reservation must live until the member is bound"]
pub struct WorkspaceReservation {
    count: Arc<AtomicUsize>,
}

impl Drop for WorkspaceReservation {
    fn drop(&mut self) {
        self.count.fetch_sub(1, Ordering::AcqRel);
    }
}

impl WorkspaceStore {
    pub fn reserve_creation(
        &self,
        address: WorkspaceAddress,
    ) -> Result<WorkspaceReservation, StoreError> {
        self.check()?;
        if self.record.find(address).is_none() {
            return Err(PublicError::NotFound.into());
        }
        let count = self
            .pending
            .lock()
            .map_err(|_| StoreError::Storage(anyhow::anyhow!("workspace reservation lock")))? 
            .entry(address)
            .or_default()
            .clone();
        count.fetch_add(1, Ordering::AcqRel);
        Ok(WorkspaceReservation { count })
    }

    pub(crate) fn creation_pending(&self, address: WorkspaceAddress) -> bool {
        self.pending
            .lock()
            .ok()
            .and_then(|pending| pending.get(&address).cloned())
            .is_some_and(|count| count.load(Ordering::Acquire) > 0)
    }
}

pub(crate) type PendingCreations = Mutex<BTreeMap<WorkspaceAddress, Arc<AtomicUsize>>>;

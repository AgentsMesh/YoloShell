use crate::model::{RuntimeRecord, WorkerHello};
use crate::worker::WorkerConnection;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{Mutex, OwnedMutexGuard};
use uuid::Uuid;

#[cfg(all(test, feature = "yolo-unit-tests"))]
#[path = "../../unit_tests/worker/worker_connections.rs"]
mod tests;

/// One authenticated stream is shared by ordinary RPCs for a runtime.
/// Attachment streams stay separate because they own an event source.
pub(crate) struct WorkerConnectionEntry {
    pub(crate) record: RuntimeRecord,
    pub(crate) hello: WorkerHello,
    pub(crate) connection: WorkerConnection,
}

pub(crate) fn identity_matches(entry: &WorkerConnectionEntry, record: &RuntimeRecord) -> bool {
    identity_matches_record(&entry.record, &entry.hello, record)
}

pub(crate) fn identity_matches_record(
    cached: &RuntimeRecord,
    hello: &WorkerHello,
    record: &RuntimeRecord,
) -> bool {
    cached.runtime_epoch == record.runtime_epoch
        && cached.controller_generation == record.controller_generation
        && hello.runtime.has_same_process_identity(record)
}

pub(crate) type WorkerConnectionSlot = Arc<Mutex<Option<WorkerConnectionEntry>>>;

#[derive(Default)]
pub(crate) struct WorkerConnections {
    pub(crate) slots: Mutex<HashMap<Uuid, WorkerConnectionSlot>>,
}

impl WorkerConnections {
    pub(crate) async fn slot(&self, terminal_id: Uuid) -> WorkerConnectionSlot {
        let mut slots = self.slots.lock().await;
        slots
            .entry(terminal_id)
            .or_insert_with(|| Arc::new(Mutex::new(None)))
            .clone()
    }

    pub(crate) async fn lock(
        &self,
        terminal_id: Uuid,
    ) -> OwnedMutexGuard<Option<WorkerConnectionEntry>> {
        self.slot(terminal_id).await.lock_owned().await
    }

    pub(crate) async fn clear(&self, terminal_id: Uuid) {
        let slot = self.slots.lock().await.remove(&terminal_id);
        if let Some(slot) = slot {
            *slot.lock().await = None;
        }
    }
}

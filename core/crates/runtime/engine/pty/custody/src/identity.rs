use crate::model::RuntimeRecord;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CustodyRuntimeIdentity {
    #[serde(with = "crate::serde_uuid")]
    pub terminal_id: Uuid,
    #[serde(with = "crate::serde_uuid")]
    pub runtime_epoch: Uuid,
    #[serde(with = "crate::serde_uuid")]
    pub worker_id: Uuid,
    pub worker_pid: u32,
    pub worker_start_time: u64,
    pub child_pid: u32,
    pub child_start_time: u64,
    pub boot_id: String,
}

#[cfg(all(test, feature = "yolo-unit-tests"))]
#[path = "../unit_tests/identity_matrix_tests.rs"]
mod matrix_tests;

impl CustodyRuntimeIdentity {
    pub fn from_record(record: &RuntimeRecord) -> Self {
        Self {
            terminal_id: record.terminal_id,
            runtime_epoch: record.runtime_epoch,
            worker_id: record.worker_id,
            worker_pid: record.worker_pid,
            worker_start_time: record.worker_start_time,
            child_pid: record.child_pid,
            child_start_time: record.child_start_time,
            boot_id: record.boot_id.clone(),
        }
    }

    pub fn matches_record(&self, record: &RuntimeRecord) -> bool {
        self == &Self::from_record(record)
    }

    pub fn is_complete(&self) -> bool {
        self.terminal_id != Uuid::nil()
            && self.runtime_epoch != Uuid::nil()
            && self.worker_id != Uuid::nil()
            && self.worker_pid != u32::default()
            && self.worker_start_time != u64::default()
            && self.child_pid != u32::default()
            && self.child_start_time != u64::default()
            && !self.boot_id.is_empty()
    }
}

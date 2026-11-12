use super::{BootstrapDelivery, runtime_capability_wire};
use crate::{ExitRecord, FaultReapStatus, TerminalRuntimeStatus};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RuntimeView {
    #[serde(with = "yolo_crypto::serde_uuid")]
    pub runtime_epoch: Uuid,
    #[serde(with = "yolo_crypto::serde_uuid")]
    pub worker_id: Uuid,
    pub worker_pid: u32,
    pub worker_start_time: u64,
    pub child_pid: u32,
    pub child_start_time: u64,
    pub status: TerminalRuntimeStatus,
    pub last_output_sequence: u64,
    pub presentation: crate::TerminalPresentation,
    pub exit: Option<ExitRecord>,
    pub bootstrap_delivery: BootstrapDelivery,
    #[serde(
        default,
        deserialize_with = "runtime_capability_wire::deserialize",
        skip_serializing_if = "Option::is_none"
    )]
    pub capabilities: Option<Vec<String>>,
    #[serde(
        default,
        deserialize_with = "super::super::fault_reap::deserialize_present",
        skip_serializing_if = "Option::is_none"
    )]
    pub fault_reap: Option<FaultReapStatus>,
}

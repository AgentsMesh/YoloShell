use serde::{Deserialize, Serialize};
use std::fs::File;
use std::path::{Path, PathBuf};
use uuid::Uuid;

#[path = "linux_containment_binding.rs"]
mod binding;
#[path = "linux_containment_broker.rs"]
mod broker;
#[path = "linux_containment_constants.rs"]
mod constants;
#[path = "linux_containment_custody.rs"]
mod custody;
#[path = "linux_containment_custody_constants.rs"]
mod custody_constants;
#[path = "linux_containment_custody_identity.rs"]
mod custody_identity;
#[path = "linux_containment_custody_validation.rs"]
mod custody_validation;
#[path = "linux_containment_descriptor.rs"]
mod descriptor;
#[path = "linux_containment_io.rs"]
mod io;
#[path = "linux_containment_membership.rs"]
mod membership;
#[path = "linux_containment_record.rs"]
mod record;
#[path = "linux_containment_recovery.rs"]
mod recovery;
#[cfg(any(test, feature = "adversarial-test-hooks"))]
#[path = "../../test_support/linux/linux_containment_resilience.rs"]
pub(in crate::process) mod resilience;
#[path = "linux_containment_settlement.rs"]
mod settlement;
#[path = "linux_containment_settlement_witness.rs"]
mod settlement_witness;
#[path = "linux_containment_transfer.rs"]
mod transfer;
#[path = "linux_containment_transition.rs"]
mod transition;
#[cfg(any(test, feature = "adversarial-test-hooks"))]
pub use resilience::ResilienceContainmentRollback;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CgroupDirectoryIdentity {
    pub mount_id: u64,
    pub inode: u64,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LinuxContainmentRecord {
    pub schema_version: u16,
    #[serde(with = "crate::serde_uuid")]
    pub terminal_id: Uuid,
    #[serde(with = "crate::serde_uuid")]
    pub runtime_epoch: Uuid,
    pub boot_id: String,
    pub delegated_root: PathBuf,
    pub root_identity: CgroupDirectoryIdentity,
    pub leaf_name: String,
    pub leaf_identity: CgroupDirectoryIdentity,
    #[serde(with = "crate::serde_uuid")]
    pub lease_token: Uuid,
    pub pty_device: Option<u64>,
    pub pty_capability: Option<String>,
    pub bound_pid: Option<u32>,
    pub bound_start_time: Option<u64>,
}

#[derive(Debug, thiserror::Error)]
pub enum LinuxContainmentError {
    #[error("Linux cgroup v2 delegation is unavailable: {0}")]
    Unavailable(&'static str),
    #[error("Linux cgroup containment identity is invalid: {0}")]
    Identity(&'static str),
    #[error("Linux cgroup containment I/O failed: {0}")]
    Io(#[from] std::io::Error),
}

pub struct LinuxContainmentBroker {
    root: File,
    root_path: PathBuf,
    root_identity: CgroupDirectoryIdentity,
}

pub struct LinuxContainmentLease {
    root: File,
    leaf: File,
    record: LinuxContainmentRecord,
    bound_root: Option<super::ProcessIdentity>,
    freeze_requested_at: Option<std::time::Instant>,
    membership_unstable_since: Option<std::time::Instant>,
}

pub(crate) use custody::LinuxContainmentCustody;
pub use recovery::{LinuxContainmentRecovery, LinuxContainmentRecoveryPoll};
pub(crate) use settlement::{LinuxContainmentCommitReady, LinuxContainmentPreparation};
pub use settlement_witness::LinuxContainmentSettlement;
pub(crate) use transfer::LinuxContainmentDescriptorExport;

impl LinuxContainmentLease {
    pub fn record(&self) -> &LinuxContainmentRecord {
        &self.record
    }

    pub(crate) fn delegated_root(&self) -> &Path {
        &self.record.delegated_root
    }

    pub(crate) fn leaf_name(&self) -> &str {
        &self.record.leaf_name
    }

    pub(crate) fn bound_root(&self) -> Option<super::ProcessIdentity> {
        self.bound_root
    }
}

pub fn record_path(session_directory: &Path, runtime_epoch: Uuid) -> PathBuf {
    constants::record_path(session_directory, runtime_epoch)
}

#[cfg(test)]
#[cfg(any(not(test), feature = "yolo-integration-tests"))]
#[path = "../../integration_tests/linux/linux_containment_record_tests.rs"]
mod record_tests;

#[cfg(test)]
#[cfg(all(target_os = "linux", feature = "linux-cgroup-kernel-test"))]
#[cfg(any(not(test), feature = "yolo-integration-tests"))]
#[path = "../../integration_tests/linux/linux_containment_kernel_tests.rs"]
mod kernel_tests;

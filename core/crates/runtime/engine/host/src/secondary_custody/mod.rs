use super::request_drain::HostRequestDrain;
use super::{Host, RuntimeRecord, SessionLocks, WorkerRpcError};
use crate::pty_custody::{
    CustodyHolderIdentity, CustodyOffer, CustodyRuntimeIdentity, CustodyUnixStream,
};
use std::collections::HashMap;
use std::sync::atomic::AtomicBool;
use std::sync::{Arc, Mutex, Weak};
use tokio::sync::Mutex as AsyncMutex;
use uuid::Uuid;

#[path = "secondary_custody_activation.rs"]
mod activation;
#[path = "secondary_custody_attachment.rs"]
mod attachment;
#[path = "secondary_custody_containment.rs"]
mod containment;
#[cfg(target_os = "macos")]
#[path = "secondary_custody_durable_retirement.rs"]
mod durable_retirement;
#[path = "secondary_custody_exchange.rs"]
mod exchange;
#[path = "secondary_custody_exchange_guard.rs"]
mod exchange_guard;
#[path = "secondary_custody_failure.rs"]
mod failure;
#[path = "secondary_custody_guardian.rs"]
mod guardian;
#[path = "secondary_custody_lease.rs"]
mod lease;
#[path = "secondary_custody_lease_platform.rs"]
mod lease_platform;
#[path = "secondary_custody_monitor.rs"]
mod monitor;
#[path = "secondary_custody_monitor_control.rs"]
mod monitor_control;
#[path = "secondary_custody_peer_validation.rs"]
mod peer_validation;
#[path = "secondary_custody_pending_release.rs"]
mod pending_release;
#[path = "secondary_custody_platform.rs"]
mod platform;
#[path = "secondary_custody_platform_state.rs"]
mod platform_state;
#[path = "secondary_custody_recovery_admission.rs"]
mod recovery_admission;
#[path = "secondary_custody_recovery_evidence.rs"]
mod recovery_evidence;
#[cfg(target_os = "linux")]
pub(in crate::host) use recovery_evidence::{RecoverySnapshot, require_gone_runtime_processes};
#[path = "secondary_custody_registry.rs"]
mod registry;
#[cfg(test)]
#[cfg(any(not(test), feature = "yolo-test-support"))]
#[path = "../../test_support/unit/secondary_custody/secondary_custody_registry_test_support.rs"]
mod registry_test_support;
#[path = "secondary_custody_types.rs"]
mod types;
pub(super) use recovery_admission::SecondaryCustodyRecoveryAdmission;
#[cfg(test)]
pub(super) use registry_test_support::TestRetiredCustodyState;
#[path = "secondary_custody_release_commit.rs"]
mod release_commit;
#[path = "secondary_custody_release_resolution.rs"]
mod release_resolution;
#[path = "secondary_custody_retirement.rs"]
mod retirement;
#[path = "secondary_custody_rollback.rs"]
mod rollback;
#[cfg(test)]
#[cfg(any(not(test), feature = "yolo-test-support"))]
#[path = "../../test_support/unit/secondary_custody/secondary_custody_rpc_test_authority.rs"]
mod rpc_test_authority;
#[path = "secondary_custody_settled_commit.rs"]
mod settled_commit;
#[path = "secondary_custody_settlement.rs"]
mod settlement;
#[path = "secondary_custody_supersede.rs"]
mod supersede;
#[path = "secondary_custody_validation.rs"]
mod validation;
use types::{
    PlatformSecondaryCustody, SecondaryCustodyAdmission, SecondaryCustodyCommand,
    SecondaryCustodyEntries, SecondaryCustodyKey, SecondaryCustodyState, SecondaryCustodyValues,
};
pub(super) use types::{SecondaryCustodyLease, SecondaryCustodyRegistry};

#[cfg(test)]
#[cfg(any(not(test), feature = "yolo-unit-tests"))]
#[path = "../../unit_tests/secondary_custody/secondary_custody_tests.rs"]
mod tests;

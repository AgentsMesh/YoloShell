use super::constants::READ_ONLY_POLL_INTERVAL;
use super::status::RelayStatusSink;
use super::{RelayError, RelayRuntime, RelayStatus};
#[cfg(all(test, feature = "yolo-unit-tests"))]
use crate::client::ClientError;
use crate::model::{WriterCapability, WriterLease, WriterLeaseGrant};
use crate::worker::RuntimeRef;
use tokio::time::Instant;
#[cfg(all(test, feature = "yolo-unit-tests"))]
use uuid::Uuid;

#[path = "lease/acquire.rs"]
mod acquire;
#[path = "lease/errors.rs"]
mod errors;
#[path = "lease/events.rs"]
mod events;
#[path = "lease/lifecycle.rs"]
mod lifecycle;
#[path = "lease/maintenance.rs"]
mod maintenance;
#[path = "lease/mode.rs"]
mod mode;
#[path = "lease/readiness.rs"]
mod readiness;
#[path = "lease/reconcile.rs"]
mod reconcile;
#[path = "lease/recovery.rs"]
mod recovery;
#[path = "lease/release.rs"]
mod release;
#[path = "lease/start.rs"]
mod start;
use errors::is_ownership_error;
pub(in crate::relay) use errors::{has_session_not_running_code, is_session_not_running};
use lifecycle::SessionLifecycle;
use mode::LeaseAcquireMode;

const INVALID_GENERATION: &str = "generation must be non-zero";
const INVALID_RENEWAL: &str = "renew_after_ms must be non-zero";
const IDENTITY_MISMATCH: &str = "holder or controller identity does not match the request";
const CAPABILITY_MISMATCH: &str = "writer capability does not match the controller";

#[cfg(all(test, feature = "yolo-unit-tests"))]
fn test_runtime_target(terminal_id: uuid::Uuid) -> RuntimeRef {
    RuntimeRef::new(
        terminal_id,
        crate::relay::session_test_support::test_runtime_epoch(),
    )
}

#[derive(Clone, Debug)]
struct LeaseIdentity {
    holder_id: String,
    controller_instance_id: String,
}

pub(crate) struct LeaseController {
    runtime: RelayRuntime,
    target: RuntimeRef,
    identity: LeaseIdentity,
    writer_capability: WriterCapability,
    lease: Option<WriterLeaseGrant>,
    conflict_holder: Option<String>,
    retry_mode: LeaseAcquireMode,
    prepared_generation: Option<u64>,
    status_announced: bool,
    next_maintenance: Instant,
    status: RelayStatusSink,
    ownership_uncertain: bool,
    control_lost: bool,
    lifecycle: SessionLifecycle,
}

impl LeaseController {
    pub(crate) fn generation(&self) -> Option<u64> {
        if self.is_terminal() {
            return None;
        }
        self.lease.as_ref().map(|grant| grant.lease.generation)
    }

    pub(crate) fn mutation_credentials(&self) -> Option<(u64, WriterCapability)> {
        if self.is_terminal() {
            return None;
        }
        self.lease
            .as_ref()
            .map(|grant| (grant.lease.generation, grant.writer_capability.clone()))
    }

    fn is_ours(&self, lease: &WriterLease) -> bool {
        lease.holder_id == self.identity.holder_id
            && lease.controller_instance_id.as_deref()
                == Some(self.identity.controller_instance_id.as_str())
    }

    fn set_writer(&mut self, grant: WriterLeaseGrant) -> Result<(), RelayError> {
        if self.is_terminal() {
            return Ok(());
        }
        if grant.writer_capability != self.writer_capability {
            return Err(RelayError::InvalidLease(CAPABILITY_MISMATCH));
        }
        if !self.is_ours(&grant.lease) {
            return Err(RelayError::InvalidLease(IDENTITY_MISMATCH));
        }
        if grant.lease.generation == 0 {
            return Err(RelayError::InvalidLease(INVALID_GENERATION));
        }
        if grant.lease.renew_after_ms == 0 {
            return Err(RelayError::InvalidLease(INVALID_RENEWAL));
        }
        let changed = self.generation() != Some(grant.lease.generation);
        self.next_maintenance =
            Instant::now() + std::time::Duration::from_millis(grant.lease.renew_after_ms);
        self.conflict_holder = None;
        self.retry_mode = LeaseAcquireMode::Observe;
        if changed {
            self.prepared_generation = None;
            self.status_announced = false;
        }
        self.lease = Some(grant);
        self.ownership_uncertain = false;
        self.control_lost = false;
        Ok(())
    }

    fn set_read_only(&mut self, holder: Option<String>, lost: bool, retry: bool) {
        if self.is_terminal() {
            return;
        }
        let held_writer = self.lease.take().is_some();
        let newly_lost = !self.control_lost && (lost || held_writer);
        self.control_lost |= lost || held_writer;
        let changed =
            !self.status_announced || held_writer || newly_lost || self.conflict_holder != holder;
        self.conflict_holder = holder.clone();
        self.retry_mode = self.retry_mode.retaining_intent(retry);
        self.prepared_generation = None;
        self.status_announced = true;
        self.schedule_read_only();
        if changed {
            let event = if self.control_lost {
                RelayStatus::ControlLost { holder_id: holder }
            } else {
                RelayStatus::ReadOnly { holder_id: holder }
            };
            self.status.send(event);
        }
    }

    fn schedule_read_only(&mut self) {
        self.next_maintenance = Instant::now() + READ_ONLY_POLL_INTERVAL;
    }

    #[cfg(test)]
    fn set_writer_for_test(&mut self, lease: WriterLease) -> Result<(), RelayError> {
        let result = self.set_writer(WriterLeaseGrant {
            lease,
            writer_capability: self.writer_capability.clone(),
        });
        if result.is_ok() {
            self.mark_generation_prepared(self.generation().unwrap());
            self.announce_control_ready();
        }
        result
    }
}

#[cfg(test)]
#[cfg(any(not(test), feature = "yolo-unit-tests"))]
include!("../unit_tests/lease_test_modules.rs");

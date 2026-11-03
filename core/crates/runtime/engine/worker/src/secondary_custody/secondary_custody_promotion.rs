use super::secondary_custody_authority::ReadyWorkerCustodyIdentity;
use super::{
    AuthenticatedController, ControllerGeneration, ErrorBody, LeaseReason, RuntimeRecord,
    SystemMutationPermit, WorkerCustodyLease, WorkerState, lock_recover,
};
use crate::paths::identity::FileIdentity;
use crate::pty_custody::CustodyDurableRecord;
use crate::pty_custody::CustodyProfile;
#[cfg(test)]
#[cfg(any(not(test), feature = "yolo-test-support"))]
#[path = "../../test_support/unit/secondary_custody/secondary_custody_promotion_test_support.rs"]
mod test_support;

const SUCCESSOR_CUSTODY_GENERATION_DETAIL: &str =
    "successor Ready PTY custody generation does not advance the worker runtime";
const SUCCESSOR_RUNTIME_CHANGED_DETAIL: &str =
    "worker runtime changed during the successor custody handshake";
const SUCCESSOR_PREDECESSOR_CHANGED_DETAIL: &str =
    "active PTY custody predecessor changed during successor adoption";
const SUCCESSOR_CANDIDATE_MISMATCH_DETAIL: &str =
    "durable PTY custody candidate does not match the exact Ready successor";
const RELEASED_SUCCESSOR_STATE_DETAIL: &str =
    "released PTY successor requires pending local closure and no Worker master source";
const RELEASED_SUCCESSOR_PROFILE_DETAIL: &str =
    "released PTY successor has an active descriptor profile";
const SUCCESSOR_PROFILE_CHANGED_DETAIL: &str =
    "successor PTY custody profile changed during adoption";
pub(crate) const SUCCESSOR_TERMINATION_INTENT_DETAIL: &str =
    "successor PTY custody cannot replace a worker with pending termination";
pub(crate) const SUCCESSOR_SHUTDOWN_COMMITTED_DETAIL: &str =
    "successor PTY custody cannot replace a shutdown-committed worker";
pub(crate) const SUCCESSOR_TERMINATION_COMMITTED_DETAIL: &str =
    "successor PTY custody cannot replace a system-terminating worker";

#[derive(Clone, Copy)]
enum SuccessorPromotionProfile {
    ActiveController,
    ActiveTerminalReadOnly,
    ReleasedRecovery,
}

impl WorkerState {
    pub(crate) fn promote_ready_custody(
        &self,
        permit: &SystemMutationPermit,
        lease: &WorkerCustodyLease,
        snapshot: &RuntimeRecord,
        predecessor: ReadyWorkerCustodyIdentity,
        durable: &CustodyDurableRecord,
        durable_identity: FileIdentity,
    ) -> std::result::Result<AuthenticatedController, ErrorBody> {
        let _child_wait_gate = self.reader_lifecycle.lock_progress();
        if lease.profile() == CustodyProfile::ActiveRunning {
            self.ensure_process_running()?;
        }
        let source_present = lock_recover(&self.secondary_custody).is_some();
        let expected = self
            .adoption_profile(source_present, false)
            .ok_or_else(|| ErrorBody::conflict(SUCCESSOR_PROFILE_CHANGED_DETAIL))?;
        if expected != lease.profile() {
            return Err(ErrorBody::conflict(SUCCESSOR_PROFILE_CHANGED_DETAIL));
        }
        let current = lock_recover(&self.record);
        if !current.has_same_process_identity(snapshot)
            || current.controller_generation != snapshot.controller_generation
        {
            return Err(ErrorBody::conflict(SUCCESSOR_RUNTIME_CHANGED_DETAIL));
        }
        if !self
            .secondary_custody_authority
            .matches_predecessor(predecessor)
        {
            return Err(ErrorBody::conflict(SUCCESSOR_PREDECESSOR_CHANGED_DETAIL));
        }
        if !lease.matches_durable_candidate(durable) {
            return Err(ErrorBody::conflict(SUCCESSOR_CANDIDATE_MISMATCH_DETAIL));
        }
        drop(current);
        let promotion_profile = match lease.profile() {
            CustodyProfile::ActiveRunning => SuccessorPromotionProfile::ActiveController,
            CustodyProfile::ActiveTerminalReadOnly => {
                SuccessorPromotionProfile::ActiveTerminalReadOnly
            }
            CustodyProfile::ReleasedTerminal => {
                return Err(ErrorBody::conflict(SUCCESSOR_PROFILE_CHANGED_DETAIL));
            }
        };
        let controller = self.promote_ready_custody_identity(
            permit,
            lease.identity(),
            promotion_profile,
            Some((durable, durable_identity)),
        )?;
        self.pty_alias_custody
            .install_release_handle(lease.release_handle());
        Ok(controller)
    }

    pub(crate) fn promote_released_ready_custody(
        &self,
        permit: &SystemMutationPermit,
        lease: &WorkerCustodyLease,
        snapshot: &RuntimeRecord,
        durable: &CustodyDurableRecord,
        durable_identity: FileIdentity,
    ) -> std::result::Result<AuthenticatedController, ErrorBody> {
        if !lease.is_released_successor() || lease.profile() != CustodyProfile::ReleasedTerminal {
            return Err(ErrorBody::conflict(RELEASED_SUCCESSOR_PROFILE_DETAIL));
        }
        let master_source_present = lock_recover(&self.secondary_custody).is_some();
        if master_source_present || !self.pty_alias_custody.released_successor_adoptable() {
            return Err(ErrorBody::conflict(RELEASED_SUCCESSOR_STATE_DETAIL));
        }
        let current = lock_recover(&self.record);
        if !current.has_same_process_identity(snapshot)
            || current.controller_generation != snapshot.controller_generation
        {
            return Err(ErrorBody::conflict(SUCCESSOR_RUNTIME_CHANGED_DETAIL));
        }
        if !lease.matches_durable_candidate(durable) {
            return Err(ErrorBody::conflict(SUCCESSOR_CANDIDATE_MISMATCH_DETAIL));
        }
        drop(current);
        let controller = self.promote_ready_custody_identity(
            permit,
            lease.identity(),
            SuccessorPromotionProfile::ReleasedRecovery,
            Some((durable, durable_identity)),
        )?;
        self.pty_alias_custody
            .install_release_handle(lease.release_handle());
        Ok(controller)
    }

    fn promote_ready_custody_identity(
        &self,
        permit: &SystemMutationPermit,
        identity: ReadyWorkerCustodyIdentity,
        profile: SuccessorPromotionProfile,
        durable: Option<(&CustodyDurableRecord, FileIdentity)>,
    ) -> std::result::Result<AuthenticatedController, ErrorBody> {
        if self.response_shutdown_is_committed() {
            return Err(ErrorBody::conflict(SUCCESSOR_SHUTDOWN_COMMITTED_DETAIL));
        }
        if matches!(
            profile,
            SuccessorPromotionProfile::ActiveController
                | SuccessorPromotionProfile::ActiveTerminalReadOnly
        ) && self.pty_alias_custody.system_termination_is_committed()
        {
            return Err(ErrorBody::conflict(SUCCESSOR_TERMINATION_COMMITTED_DETAIL));
        }
        if matches!(
            profile,
            SuccessorPromotionProfile::ActiveController
                | SuccessorPromotionProfile::ActiveTerminalReadOnly
        ) && self.has_termination_intent()
        {
            return Err(ErrorBody::conflict(SUCCESSOR_TERMINATION_INTENT_DETAIL));
        }
        let generation = identity.generation();
        let current = self
            .controller_generation
            .load(std::sync::atomic::Ordering::Acquire);
        let persisted = self
            .controller_persisted_generation
            .load(std::sync::atomic::Ordering::Acquire);
        if generation <= current || generation <= persisted {
            return Err(ErrorBody::conflict(SUCCESSOR_CUSTODY_GENERATION_DETAIL));
        }
        let controller = AuthenticatedController {
            generation: ControllerGeneration(generation),
        };
        match durable {
            Some((record, identity)) => {
                self.persist_custody_controller_generation(controller.generation, record, identity)?
            }
            None => self.persist_controller_generation(controller.generation)?,
        }
        self.revoke_writer_lease_admitted(permit, LeaseReason::ControllerReplaced);
        match profile {
            SuccessorPromotionProfile::ActiveController => {
                self.secondary_custody_authority
                    .promote(identity, CustodyProfile::ActiveRunning);
            }
            SuccessorPromotionProfile::ActiveTerminalReadOnly => {
                self.secondary_custody_authority
                    .promote(identity, CustodyProfile::ActiveTerminalReadOnly);
            }
            SuccessorPromotionProfile::ReleasedRecovery => {
                self.assume_released_recovery_termination_authority(permit);
            }
        }
        Ok(controller)
    }
}

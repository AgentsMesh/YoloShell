pub(crate) use super::secondary_custody_identity::ReadyWorkerCustodyIdentity;
use super::{
    AuthenticatedController, ControllerGeneration, ErrorBody, SystemMutationPermit,
    WorkerCustodyLease, WorkerState, lock_recover,
};
use crate::pty_custody::CustodyProfile;
use std::sync::{Arc, Mutex};

#[path = "secondary_custody_authority_fence.rs"]
mod fence;
#[path = "secondary_custody_authority_profile.rs"]
mod profile;
#[path = "secondary_custody_reconciliation.rs"]
pub(crate) mod reconciliation;

pub(crate) const CUSTODY_UNAVAILABLE_DETAIL: &str =
    "controller generation has no active Ready PTY custody lease";
const INITIAL_CUSTODY_GENERATION_DETAIL: &str =
    "initial Ready PTY custody generation does not match the worker runtime";
const INITIAL_CUSTODY_CONFLICT_DETAIL: &str =
    "initial Ready PTY custody conflicts with an active lease";

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
enum WorkerSecondaryCustodyAuthorityState {
    #[default]
    Vacant,
    Ready {
        identity: ReadyWorkerCustodyIdentity,
        profile: CustodyProfile,
    },
    Disconnected {
        identity: ReadyWorkerCustodyIdentity,
        profile: CustodyProfile,
    },
}

#[derive(Default)]
pub(crate) struct WorkerSecondaryCustodyAuthority {
    state: Mutex<WorkerSecondaryCustodyAuthorityState>,
    pending: Mutex<Option<reconciliation::PendingCustodyReconciliation>>,
}

impl WorkerSecondaryCustodyAuthority {
    fn authorizes(&self, generation: ControllerGeneration) -> bool {
        matches!(
            *lock_recover(&self.state),
            WorkerSecondaryCustodyAuthorityState::Ready { identity, .. }
                if identity.controller_generation() == generation
        )
    }

    fn install_initial(
        &self,
        identity: ReadyWorkerCustodyIdentity,
    ) -> std::result::Result<(), ErrorBody> {
        let mut state = lock_recover(&self.state);
        match *state {
            WorkerSecondaryCustodyAuthorityState::Vacant => {
                *state = WorkerSecondaryCustodyAuthorityState::Ready {
                    identity,
                    profile: CustodyProfile::ActiveRunning,
                };
                Ok(())
            }
            WorkerSecondaryCustodyAuthorityState::Ready {
                identity: current, ..
            } if current == identity => Ok(()),
            _ => Err(ErrorBody::conflict(INITIAL_CUSTODY_CONFLICT_DETAIL)),
        }
    }

    pub(crate) fn promote(&self, identity: ReadyWorkerCustodyIdentity, profile: CustodyProfile) {
        *lock_recover(&self.state) =
            WorkerSecondaryCustodyAuthorityState::Ready { identity, profile };
    }

    fn mark_unavailable(&self, identity: ReadyWorkerCustodyIdentity) {
        let mut state = lock_recover(&self.state);
        if matches!(*state, WorkerSecondaryCustodyAuthorityState::Ready { identity: current, .. } if current == identity)
        {
            let profile = match *state {
                WorkerSecondaryCustodyAuthorityState::Ready { profile, .. } => profile,
                _ => unreachable!(),
            };
            *state = WorkerSecondaryCustodyAuthorityState::Disconnected { identity, profile };
        }
    }

    pub(crate) fn predecessor_identity(&self) -> Option<ReadyWorkerCustodyIdentity> {
        match *lock_recover(&self.state) {
            WorkerSecondaryCustodyAuthorityState::Ready { identity, .. }
            | WorkerSecondaryCustodyAuthorityState::Disconnected { identity, .. } => Some(identity),
            WorkerSecondaryCustodyAuthorityState::Vacant => None,
        }
    }

    pub(crate) fn matches_predecessor(&self, identity: ReadyWorkerCustodyIdentity) -> bool {
        self.predecessor_identity() == Some(identity)
    }

    fn fence_all(&self) {
        *lock_recover(&self.state) = WorkerSecondaryCustodyAuthorityState::Vacant;
    }

    #[cfg(test)]
    pub(crate) fn ready_for_test(generation: u64) -> Self {
        Self {
            state: Mutex::new(WorkerSecondaryCustodyAuthorityState::Ready {
                identity: ReadyWorkerCustodyIdentity::fresh_for_test(generation),
                profile: CustodyProfile::ActiveRunning,
            }),
            ..Self::default()
        }
    }

    #[cfg(test)]
    pub(crate) fn active_for_test(&self) -> Option<ReadyWorkerCustodyIdentity> {
        self.predecessor_identity()
    }

    #[cfg(test)]
    pub(crate) fn disconnected_for_test(&self, identity: ReadyWorkerCustodyIdentity) -> bool {
        matches!(
            *lock_recover(&self.state),
            WorkerSecondaryCustodyAuthorityState::Disconnected { identity: current, .. }
                if current == identity
        )
    }

    #[cfg(test)]
    pub(crate) fn replace_ready_for_test(&self, identity: ReadyWorkerCustodyIdentity) {
        let profile = identity.profile();
        *lock_recover(&self.state) =
            WorkerSecondaryCustodyAuthorityState::Ready { identity, profile };
    }
}

impl WorkerState {
    pub(crate) async fn authenticate_ready_controller(
        self: &Arc<Self>,
        generation: u64,
    ) -> std::result::Result<AuthenticatedController, ErrorBody> {
        let _permit = self.admit_system_mutation().await;
        let controller = AuthenticatedController {
            generation: ControllerGeneration(generation),
        };
        self.validate_controller(controller)?;
        Ok(controller)
    }

    pub(crate) fn install_initial_ready_custody(
        &self,
        _permit: &SystemMutationPermit,
        lease: &WorkerCustodyLease,
    ) -> std::result::Result<(), ErrorBody> {
        let identity = lease.identity();
        if identity.generation()
            != self
                .controller_generation
                .load(std::sync::atomic::Ordering::Acquire)
        {
            return Err(ErrorBody::conflict(INITIAL_CUSTODY_GENERATION_DETAIL));
        }
        self.secondary_custody_authority.install_initial(identity)?;
        self.pty_alias_custody
            .install_release_handle(lease.release_handle());
        Ok(())
    }

    pub(crate) async fn mark_custody_unavailable(
        self: &Arc<Self>,
        identity: ReadyWorkerCustodyIdentity,
    ) {
        let _permit = self.admit_system_mutation().await;
        self.secondary_custody_authority.mark_unavailable(identity);
        self.pty_alias_custody.clear_release_handle(identity);
        #[cfg(test)]
        self.control_test_hooks
            .custody_after_unavailable
            .pause_if_armed()
            .await;
    }

    pub(crate) fn custody_authorizes(&self, generation: ControllerGeneration) -> bool {
        self.secondary_custody_authority.authorizes(generation)
    }

    pub(crate) async fn fence_all_custody(self: &Arc<Self>) {
        let _permit = self.admit_system_mutation().await;
        self.fence_all_custody_now();
    }

    pub(crate) fn fence_all_custody_now(&self) {
        self.secondary_custody_authority.fence_all();
        self.pty_alias_custody.clear_all_release_handles();
    }
}

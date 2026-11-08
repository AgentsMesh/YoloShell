use super::{
    ReadyWorkerCustodyIdentity, WorkerSecondaryCustodyAuthority,
    WorkerSecondaryCustodyAuthorityState,
};
use crate::worker::{WorkerState, lock_recover};

impl WorkerSecondaryCustodyAuthority {
    pub(crate) fn fence_failed_adoption(&self, predecessor: Option<ReadyWorkerCustodyIdentity>) {
        let mut state = lock_recover(&self.state);
        *state = match (*state, predecessor) {
            (
                WorkerSecondaryCustodyAuthorityState::Ready { identity, profile }
                | WorkerSecondaryCustodyAuthorityState::Disconnected { identity, profile },
                Some(expected),
            ) if identity == expected => {
                WorkerSecondaryCustodyAuthorityState::Disconnected { identity, profile }
            }
            _ => WorkerSecondaryCustodyAuthorityState::Vacant,
        };
    }
}

impl WorkerState {
    pub(crate) fn fence_failed_adoption_now(
        &self,
        predecessor: Option<ReadyWorkerCustodyIdentity>,
    ) {
        self.secondary_custody_authority
            .fence_failed_adoption(predecessor);
        self.pty_alias_custody.clear_all_release_handles();
    }
}

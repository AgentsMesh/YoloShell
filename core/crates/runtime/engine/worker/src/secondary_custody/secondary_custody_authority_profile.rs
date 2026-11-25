use super::super::lock_recover;
use super::{
    ControllerGeneration, WorkerSecondaryCustodyAuthority, WorkerSecondaryCustodyAuthorityState,
};
use crate::pty_custody::CustodyProfile;

impl WorkerSecondaryCustodyAuthority {
    pub(crate) fn profile_for(&self, generation: ControllerGeneration) -> Option<CustodyProfile> {
        match *lock_recover(&self.state) {
            WorkerSecondaryCustodyAuthorityState::Ready { identity, profile }
            | WorkerSecondaryCustodyAuthorityState::Disconnected { identity, profile }
                if identity.controller_generation() == generation =>
            {
                Some(profile)
            }
            _ => None,
        }
    }
}

impl super::WorkerState {
    pub(crate) fn custody_profile(
        &self,
        generation: ControllerGeneration,
    ) -> Option<CustodyProfile> {
        self.secondary_custody_authority.profile_for(generation)
    }

    pub(crate) fn adoption_profile(
        &self,
        source_present: bool,
        released: bool,
    ) -> Option<CustodyProfile> {
        if released {
            return Some(CustodyProfile::ReleasedTerminal);
        }
        source_present.then(|| {
            if self.child_waited.load(std::sync::atomic::Ordering::Acquire) {
                CustodyProfile::ActiveTerminalReadOnly
            } else {
                CustodyProfile::ActiveRunning
            }
        })
    }
}

use super::StartupChildGuard;
use crate::worker::startup::startup_untracked_reaper::StartupUntrackedIdentity;

impl StartupChildGuard {
    pub(crate) fn take_untracked_identity(&mut self) -> StartupUntrackedIdentity {
        match (self.identity.take(), self.live_identity.take()) {
            (Some(durable), Some(live)) => StartupUntrackedIdentity::exact(durable, live),
            (durable, live) => StartupUntrackedIdentity::incomplete(durable, live),
        }
    }
}

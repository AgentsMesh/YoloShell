use super::{LeaseAcquireMode, LeaseController};
use crate::client::ClientError;
use crate::relay::RelayStatus;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum SessionLifecycle {
    Active,
    Ending,
}

impl LeaseController {
    pub(crate) fn is_terminal(&self) -> bool {
        !matches!(self.lifecycle, SessionLifecycle::Active)
    }

    pub(super) fn observe_session_ending(&mut self, error: &ClientError) -> bool {
        if !super::is_session_not_running(error) {
            return false;
        }
        if matches!(self.lifecycle, SessionLifecycle::Active) {
            self.lifecycle = SessionLifecycle::Ending;
            self.fence_terminal_authority();
            self.status.send(RelayStatus::SessionEnding);
        }
        true
    }

    pub(super) fn observe_session_ended(&mut self) {
        if matches!(self.lifecycle, SessionLifecycle::Active) {
            self.lifecycle = SessionLifecycle::Ending;
            self.fence_terminal_authority();
            self.status.send(RelayStatus::SessionEnding);
        }
    }

    fn fence_terminal_authority(&mut self) {
        self.lease = None;
        self.conflict_holder = None;
        self.retry_mode = LeaseAcquireMode::Disabled;
        self.prepared_generation = None;
        self.status_announced = true;
        self.ownership_uncertain = false;
        self.control_lost = false;
    }
}

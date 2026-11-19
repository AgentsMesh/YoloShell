use super::CodexRouterError;
use super::state::{RouteRetirementState, RouteSlot};

impl RouteSlot {
    pub(super) fn request_retirement(&mut self) -> Result<bool, CodexRouterError> {
        if !self.committed || self.rollback_pending {
            return Err(CodexRouterError::Conflict);
        }
        if self.retirement == RouteRetirementState::Active {
            self.retirement = RouteRetirementState::Pending;
        }
        Ok(self.resume_retirement().unwrap_or(false))
    }

    pub(super) fn resume_retirement(&mut self) -> Option<bool> {
        if self.retirement == RouteRetirementState::Active {
            return None;
        }
        if self.begin_leases != usize::default() || self.retirement != RouteRetirementState::Pending
        {
            return Some(false);
        }
        self.retirement = RouteRetirementState::Permitted;
        Some(true)
    }

    pub(super) fn commit_retirement(&self) -> Result<(), CodexRouterError> {
        if self.begin_leases == usize::default()
            && self.retirement == RouteRetirementState::Permitted
        {
            Ok(())
        } else {
            Err(CodexRouterError::Conflict)
        }
    }

    pub(super) fn verify_retirement_permit(&self) -> Result<(), CodexRouterError> {
        self.commit_retirement()
    }

    pub(super) fn release_retirement_permit(&mut self) {
        if self.retirement == RouteRetirementState::Permitted {
            self.retirement = RouteRetirementState::Pending;
        }
    }
}

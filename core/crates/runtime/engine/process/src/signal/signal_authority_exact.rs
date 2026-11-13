use super::{DestructiveSessionAuthority, ProcessSignalAuthority};
use crate::process::LiveProcessIdentity;
use anyhow::Result;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(in crate::process) enum ExactSignalOutcome {
    Delivered,
    ConfirmedAbsent,
    IdentityReplaced,
    Undelivered,
}

impl ExactSignalOutcome {
    pub(in crate::process) fn authorizes_resolution(self) -> bool {
        matches!(self, Self::Delivered | Self::ConfirmedAbsent)
    }
}

impl DestructiveSessionAuthority {
    pub(in crate::process) fn signal_exact_sigkill(
        &self,
        identity: &LiveProcessIdentity,
    ) -> Result<ExactSignalOutcome> {
        self.0.signal_exact_sigkill(identity)
    }
}

impl ProcessSignalAuthority {
    fn signal_exact_sigkill(&self, identity: &LiveProcessIdentity) -> Result<ExactSignalOutcome> {
        identity.signal_exact_sigkill_authorized(self)
    }
}

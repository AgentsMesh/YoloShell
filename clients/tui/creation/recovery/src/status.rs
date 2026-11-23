use crate::{BlockedReason, CreationRecovery};

/// Disjoint buckets sum to len(). Pending waits for a retry or connection;
/// resolved evidence waits for the caller's durable journal acknowledgement.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct RecoveryStatus {
    pub pending: usize,
    pub in_flight: usize,
    pub resolved: usize,
    pub blocked: usize,
    pub first_blocked: Option<BlockedReason>,
}

impl CreationRecovery {
    pub fn status(&self) -> RecoveryStatus {
        let mut status = RecoveryStatus::default();
        for entry in self.entries.values() {
            if entry.resolution.is_some() {
                status.resolved += 1;
            } else if let Some(reason) = entry.blocked {
                status.blocked += 1;
                status.first_blocked.get_or_insert(reason);
            } else if entry.attempt.is_some() {
                status.in_flight += 1;
            } else {
                status.pending += 1;
            }
        }
        status
    }
}

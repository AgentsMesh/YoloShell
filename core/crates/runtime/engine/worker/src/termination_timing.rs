pub(super) use crate::session_timing::{
    GRACEFUL_TERMINATION_POLL_INTERVAL, GRACEFUL_TERMINATION_TIMEOUT,
    PTY_CAPABILITY_SETTLEMENT_TIMEOUT, PTY_MISSING_EOF_TIMEOUT, WORKER_AUTH_TIMEOUT,
    WORKER_TERMINATION_PROTOCOL_TIMEOUT as WORKER_TERMINATION_TIMEOUT,
};
use std::fmt::{Display, Formatter};
use std::time::Instant;

const CAPABILITY_SETTLEMENT_DEFERRED_DETAIL: &str =
    "PTY capability settlement was deferred to the fault reaper";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) struct CapabilitySettlementDeferred;

#[derive(Clone, Copy)]
pub(super) struct CapabilitySettlementDeadline {
    expires_at: Instant,
}

impl CapabilitySettlementDeadline {
    pub(super) fn production(now: Instant) -> Self {
        Self::with_timeout(now, PTY_CAPABILITY_SETTLEMENT_TIMEOUT)
    }

    pub(super) fn expired_at(self, now: Instant) -> bool {
        now >= self.expires_at
    }

    fn with_timeout(now: Instant, timeout: std::time::Duration) -> Self {
        Self {
            expires_at: now + timeout,
        }
    }

    #[cfg(test)]
    pub(super) fn immediate(now: Instant) -> Self {
        Self::with_timeout(now, std::time::Duration::ZERO)
    }
}

impl Display for CapabilitySettlementDeferred {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(CAPABILITY_SETTLEMENT_DEFERRED_DETAIL)
    }
}

impl std::error::Error for CapabilitySettlementDeferred {}
#[cfg(test)]
pub(super) use crate::session_timing::{
    FORCE_TERMINATION_CONFIRM_TIMEOUT, FORCE_TERMINATION_TIMEOUT,
    MAXIMUM_SCHEDULED_TERMINATION_DRAIN, PROCESS_SCAN_DEADLINE_OVERRUN_MARGIN,
    WORKER_AUTH_RESPONSE_MARGIN,
};

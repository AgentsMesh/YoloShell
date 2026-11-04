use std::time::Duration;
use tokio::time::Instant;

const INITIAL_FAILURE_LOG_BACKOFF: Duration = crate::session_timing::GRACEFUL_TERMINATION_TIMEOUT;
const MAXIMUM_FAILURE_LOG_BACKOFF: Duration =
    crate::session_timing::WORKER_TERMINATION_PROTOCOL_TIMEOUT;

pub(super) struct FailureLogBackoff {
    delay: Duration,
    next_emit: Option<Instant>,
}

impl FailureLogBackoff {
    pub(super) fn should_emit(&mut self, now: Instant) -> bool {
        if self.next_emit.is_some_and(|deadline| now < deadline) {
            return false;
        }
        self.next_emit = Some(now + self.delay);
        self.delay = self
            .delay
            .saturating_add(self.delay)
            .min(MAXIMUM_FAILURE_LOG_BACKOFF);
        true
    }

    #[cfg(test)]
    fn delay(&self) -> Duration {
        self.delay
    }
}

impl Default for FailureLogBackoff {
    fn default() -> Self {
        Self {
            delay: INITIAL_FAILURE_LOG_BACKOFF,
            next_emit: None,
        }
    }
}

#[cfg(test)]
#[cfg(feature = "yolo-unit-tests")]
#[path = "../../unit_tests/settlement/settlement_backoff_inline_tests.rs"]
mod tests;

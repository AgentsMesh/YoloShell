use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{OwnedSemaphorePermit, Semaphore};

const ACCEPT_BACKOFF_MULTIPLIER: u32 = 2;

pub struct AcceptBackoff {
    initial: Duration,
    maximum: Duration,
    failures: u32,
}

impl AcceptBackoff {
    pub fn new(initial: Duration, maximum: Duration) -> Self {
        Self {
            initial,
            maximum,
            failures: 0,
        }
    }

    pub fn reset(&mut self) {
        self.failures = 0;
    }

    pub fn next_delay(&mut self) -> Duration {
        let exponent = self.failures.min(u32::BITS - 1);
        self.failures = self.failures.saturating_add(1);
        let factor = ACCEPT_BACKOFF_MULTIPLIER.saturating_pow(exponent);
        self.initial.saturating_mul(factor).min(self.maximum)
    }

    #[cfg(any(test, feature = "yolo-test-support"))]
    pub fn failures(&self) -> u32 {
        self.failures
    }
}

pub fn try_admit(clients: &Arc<Semaphore>) -> Option<OwnedSemaphorePermit> {
    Arc::clone(clients).try_acquire_owned().ok()
}

#[cfg(test)]
#[cfg(feature = "yolo-unit-tests")]
#[path = "../unit_tests/mod_inline_tests.rs"]
mod tests;

use std::time::{Duration, Instant};

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(crate) enum FaultReapActivity {
    #[default]
    Idle,
    ScanContinuation,
    SemanticProgress,
}

impl FaultReapActivity {
    pub(crate) fn with_semantic_progress(self, progressed: bool) -> Self {
        if progressed {
            Self::SemanticProgress
        } else {
            self
        }
    }
}

#[derive(Clone, Copy)]
pub(crate) struct FaultReapSchedule {
    pub(crate) prompt_deadline: Instant,
}

impl FaultReapSchedule {
    pub(crate) fn production() -> Self {
        Self::with_timeout(
            Instant::now(),
            crate::worker::termination_timing::WORKER_TERMINATION_TIMEOUT,
        )
    }

    pub(crate) fn pause(
        self,
        current: Duration,
        activity: FaultReapActivity,
        initial: Duration,
        maximum: Duration,
    ) -> Duration {
        let (wait, next) = self.retry_at(current, activity, initial, maximum, Instant::now());
        if !wait.is_zero() {
            std::thread::sleep(wait);
        }
        next
    }

    pub(crate) fn retry_at(
        self,
        current: Duration,
        activity: FaultReapActivity,
        initial: Duration,
        maximum: Duration,
        now: Instant,
    ) -> (Duration, Duration) {
        let prompt_limit = crate::worker::termination_timing::GRACEFUL_TERMINATION_POLL_INTERVAL;
        let prompt = self.prompt_deadline > now;
        let initial = if prompt {
            initial.min(prompt_limit)
        } else {
            initial
        };
        let current = if prompt {
            current.min(prompt_limit)
        } else {
            current
        };
        let maximum = if prompt {
            maximum.min(prompt_limit)
        } else {
            maximum
        };
        let requested = match (prompt, activity) {
            (_, FaultReapActivity::ScanContinuation)
            | (true, FaultReapActivity::SemanticProgress) => Duration::ZERO,
            (false, FaultReapActivity::SemanticProgress) => initial,
            (_, FaultReapActivity::Idle) => current,
        };
        let next = match activity {
            FaultReapActivity::SemanticProgress => initial,
            FaultReapActivity::ScanContinuation => current,
            FaultReapActivity::Idle => current
                .saturating_mul(super::constants::PTY_FAULT_REAPER_RETRY_MULTIPLIER)
                .min(maximum),
        };
        (self.clamp_wait(requested, now), next)
    }

    pub(crate) fn clamp_wait(self, requested: Duration, now: Instant) -> Duration {
        let Some(remaining) = self.prompt_deadline.checked_duration_since(now) else {
            return requested;
        };
        if remaining.is_zero() {
            return requested;
        }
        requested
            .min(crate::worker::termination_timing::GRACEFUL_TERMINATION_POLL_INTERVAL)
            .min(remaining)
    }

    pub(crate) fn with_timeout(now: Instant, timeout: Duration) -> Self {
        Self {
            prompt_deadline: now + timeout,
        }
    }

    #[cfg(test)]
    pub(crate) fn expired_for_test() -> Self {
        Self::with_timeout(Instant::now(), Duration::ZERO)
    }

    #[cfg(test)]
    pub(crate) fn is_prompt_for_test(self) -> bool {
        self.prompt_deadline > Instant::now()
    }
}

#[cfg(test)]
#[cfg(any(not(test), feature = "yolo-unit-tests"))]
#[cfg(any(feature = "yolo-integration-tests", feature = "yolo-worker-unit-runtime"))]
#[path = "../../../../unit_tests/pty_fault_reaper/schedule_tests.rs"]
mod tests;

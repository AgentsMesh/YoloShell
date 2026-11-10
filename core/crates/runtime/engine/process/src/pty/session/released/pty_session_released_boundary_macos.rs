use crate::process::constants::MAX_PTY_PROOF_CANDIDATES;
use std::time::{Duration, Instant};

// A candidate's descriptor inspection is indivisible. Live scans yield after
// one expensive candidate. A frozen termination scan has a bounded poll budget,
// so it must retire a larger minimum batch before yielding to its time slice.
pub(super) const RELEASED_LIVE_SWEEP_MINIMUM_WORK_PER_POLL: usize = 1;
pub(super) const RELEASED_FROZEN_SWEEP_MAX_POLL_COUNT: usize = 256;
pub(super) const RELEASED_FROZEN_SWEEP_MINIMUM_WORK_PER_POLL: usize =
    MAX_PTY_PROOF_CANDIDATES.div_ceil(RELEASED_FROZEN_SWEEP_MAX_POLL_COUNT);
const RELEASED_SWEEP_TIME_SLICE: Duration = Duration::from_millis(10);

#[derive(Clone, Copy)]
pub(super) struct ReleasedPollBoundary {
    work_limit: usize,
    minimum_work: usize,
    started_at: Option<Instant>,
    time_slice: Option<Duration>,
}

impl ReleasedPollBoundary {
    pub(super) fn live_production() -> Self {
        Self::production(RELEASED_LIVE_SWEEP_MINIMUM_WORK_PER_POLL)
    }

    pub(super) fn frozen_production() -> Self {
        Self::production(RELEASED_FROZEN_SWEEP_MINIMUM_WORK_PER_POLL)
    }

    fn production(minimum_work: usize) -> Self {
        Self {
            work_limit: MAX_PTY_PROOF_CANDIDATES,
            minimum_work,
            started_at: None,
            time_slice: Some(RELEASED_SWEEP_TIME_SLICE),
        }
    }

    #[cfg(test)]
    pub(super) fn work_only(work_limit: usize) -> Self {
        Self {
            work_limit,
            minimum_work: usize::default(),
            started_at: Some(Instant::now()),
            time_slice: None,
        }
    }

    #[cfg(test)]
    pub(super) fn with_time_slice(
        work_limit: usize,
        minimum_work: usize,
        time_slice: Duration,
    ) -> Self {
        assert!(minimum_work <= work_limit);
        Self {
            work_limit,
            minimum_work,
            started_at: Some(Instant::now()),
            time_slice: Some(time_slice),
        }
    }

    pub(super) fn begin_candidate_work(mut self) -> Self {
        self.started_at.get_or_insert_with(Instant::now);
        self
    }

    pub(super) fn remaining_work(self, consumed: usize) -> usize {
        self.work_limit.saturating_sub(consumed)
    }

    pub(super) fn time_expired(self, consumed: usize) -> bool {
        consumed >= self.minimum_work
            && self
                .time_slice
                .zip(self.started_at)
                .is_some_and(|(limit, started_at)| started_at.elapsed() >= limit)
    }
}

#[cfg(test)]
#[cfg(feature = "yolo-unit-tests")]
#[path = "../../../../unit_tests/pty_session_released_boundary_macos_inline_tests.rs"]
mod tests;

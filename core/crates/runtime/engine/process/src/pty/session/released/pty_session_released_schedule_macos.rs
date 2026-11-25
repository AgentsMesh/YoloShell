use super::universe::ReleasedCandidate;
use crate::process::LiveProcessIdentity;
use std::cmp::Reverse;
use std::collections::{HashSet, VecDeque};

pub(super) fn ordered_schedule(
    current: &[ReleasedCandidate],
    exact_priorities: &HashSet<LiveProcessIdentity>,
) -> Vec<ReleasedCandidate> {
    // Priority changes work order only; every candidate still passes the exact
    // generation and descriptor fences in evaluate_global_candidate.
    let mut exact = sorted_lane(current, None, exact_priorities);
    let mut eligible = sorted_lane(current, Some(true), exact_priorities);
    let mut fallback = sorted_lane(current, Some(false), exact_priorities);
    let mut scheduled = Vec::with_capacity(current.len());
    scheduled.extend(exact.drain(..));
    while !eligible.is_empty() || !fallback.is_empty() {
        scheduled.extend(eligible.pop_front());
        scheduled.extend(fallback.pop_front());
    }
    scheduled
}

fn sorted_lane(
    current: &[ReleasedCandidate],
    eligible: Option<bool>,
    exact_priorities: &HashSet<LiveProcessIdentity>,
) -> VecDeque<ReleasedCandidate> {
    let mut lane = current
        .iter()
        .copied()
        .filter(|candidate| match eligible {
            None => exact_priorities.contains(&candidate.live),
            Some(eligible) => {
                !exact_priorities.contains(&candidate.live)
                    && candidate.environment_eligible == eligible
            }
        })
        .collect::<Vec<_>>();
    lane.sort_unstable_by_key(|candidate| {
        (
            Reverse(candidate.durable.start_time),
            Reverse(candidate.live.pid()),
            candidate.live.unique_id(),
        )
    });
    lane.into()
}

#[cfg(test)]
#[cfg(any(not(test), feature = "yolo-integration-tests"))]
#[path = "../../../../integration_tests/pty/session/released/pty_session_released_schedule_tests.rs"]
mod tests;

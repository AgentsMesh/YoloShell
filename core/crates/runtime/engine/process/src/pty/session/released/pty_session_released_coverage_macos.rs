use super::queue::{QueueTake, ReleasedSweepQueues, ScheduledCandidate};
use super::universe::ReleasedCandidate;
use crate::process::LiveProcessIdentity;
use anyhow::Result;
use std::collections::{HashMap, HashSet};

const ROUND_PRESENT_DETAIL: &str = "released round is present";
#[derive(Clone, Copy, Default, Eq, PartialEq)]
pub(super) enum ReleasedSweepPolicy {
    #[default]
    Live,
    // Fork sources are stopped before this snapshot is trusted. New arrivals wait
    // for the post-stop event fence to request a fresh round.
    Frozen,
}

#[derive(Clone, Default)]
pub(super) struct ReleasedSweepCoverage {
    round: Option<HashMap<LiveProcessIdentity, ReleasedCandidate>>,
    completed: Option<CompletedReleasedSweep>,
    remaining: HashSet<LiveProcessIdentity>,
    deferred: HashSet<LiveProcessIdentity>,
    prioritized: HashSet<LiveProcessIdentity>,
    // Queue entries are lazy tombstones. Exact round membership decides whether
    // an entry is current; the bounded scheduler gives old and new work fair turns.
    queues: ReleasedSweepQueues,
    policy: ReleasedSweepPolicy,
}

#[derive(Clone)]
struct CompletedReleasedSweep {
    candidates: HashMap<LiveProcessIdentity, ReleasedCandidate>,
    deferred: HashSet<LiveProcessIdentity>,
    policy: ReleasedSweepPolicy,
}

impl ReleasedSweepCoverage {
    #[cfg(all(feature = "adversarial-test-hooks", not(test)))]
    pub(super) fn adversarial_snapshot(&self) -> String {
        let mut remaining = self
            .remaining
            .iter()
            .map(LiveProcessIdentity::pid)
            .collect::<Vec<_>>();
        remaining.sort_unstable();
        let mut deferred = self
            .deferred
            .iter()
            .map(LiveProcessIdentity::pid)
            .collect::<Vec<_>>();
        deferred.sort_unstable();
        let policy = match self.policy {
            ReleasedSweepPolicy::Live => "live",
            ReleasedSweepPolicy::Frozen => "frozen",
        };
        format!(
            "policy={policy} round={} remaining={remaining:?} deferred={deferred:?} queued={}",
            self.round.as_ref().map_or(usize::default(), HashMap::len),
            self.queues.len(),
        )
    }

    pub(super) fn begin_round(
        &mut self,
        current: &[ReleasedCandidate],
        policy: ReleasedSweepPolicy,
        exact_priorities: &HashSet<LiveProcessIdentity>,
    ) -> Result<(bool, bool, bool, bool)> {
        let candidates = current
            .iter()
            .map(|candidate| (candidate.live, *candidate))
            .collect();
        if self.round.is_none() || self.policy != policy {
            return self.begin_new_round(candidates, current, policy, exact_priorities);
        }
        let (changed, scheduled) = self.reconcile_round(candidates, current, exact_priorities)?;
        self.promote_exact(current, exact_priorities, &scheduled)?;
        let (scheduled, conclusive) = self.complete_if_empty();
        Ok((scheduled, conclusive, changed, changed))
    }

    pub(super) fn take_next(&mut self, work_limit: usize) -> QueueTake {
        let Some(round) = self.round.as_ref() else {
            return QueueTake::Exhausted(usize::default());
        };
        self.queues.take_next(round, &self.remaining, work_limit)
    }

    pub(super) fn record_attempt(
        &mut self,
        scheduled: ScheduledCandidate,
        conclusive: bool,
    ) -> (bool, bool) {
        let identity = scheduled.candidate.live;
        if conclusive {
            self.deferred.remove(&identity);
        } else {
            self.deferred.insert(identity);
        }
        self.remaining.remove(&identity);
        self.complete_if_empty()
    }

    pub(super) fn restore_retry(&mut self, scheduled: ScheduledCandidate) {
        let identity = scheduled.candidate.live;
        let current = self.round.as_ref().and_then(|round| round.get(&identity));
        if current != Some(&scheduled.candidate) || !self.remaining.contains(&identity) {
            return;
        }
        self.queues.restore_retry(scheduled);
    }

    pub(super) fn conclusive_observation_is_new(&self, identity: &LiveProcessIdentity) -> bool {
        self.deferred.contains(identity)
            || self
                .completed
                .as_ref()
                .is_some_and(|completed| completed.deferred.contains(identity))
    }

    fn finish_round(&mut self) {
        if let Some(candidates) = self.round.take() {
            self.completed = Some(CompletedReleasedSweep {
                candidates,
                deferred: std::mem::take(&mut self.deferred),
                policy: self.policy,
            });
        }
        self.clear_round();
    }

    fn discard_round(&mut self) {
        self.round = None;
        self.clear_round();
    }

    fn clear_round(&mut self) {
        self.remaining.clear();
        self.deferred.clear();
        self.prioritized.clear();
        self.queues.clear();
        self.policy = ReleasedSweepPolicy::default();
    }

    fn start_round(
        &mut self,
        candidates: HashMap<LiveProcessIdentity, ReleasedCandidate>,
        ordered: &[ReleasedCandidate],
        exact_priorities: &HashSet<LiveProcessIdentity>,
    ) -> Result<()> {
        self.queues.start(ordered)?;
        self.remaining = candidates.keys().copied().collect();
        self.prioritized.clear();
        self.round = Some(candidates);
        self.promote_exact(ordered, exact_priorities, &HashSet::new())
    }

    fn promote_exact(
        &mut self,
        ordered: &[ReleasedCandidate],
        exact_priorities: &HashSet<LiveProcessIdentity>,
        scheduled: &HashSet<LiveProcessIdentity>,
    ) -> Result<()> {
        let mut additions = Vec::new();
        for candidate in ordered.iter().copied() {
            if !exact_priorities.contains(&candidate.live) {
                continue;
            }
            let newly_prioritized = self.prioritized.insert(candidate.live);
            let retry_deferred = self.deferred.contains(&candidate.live)
                && !self.remaining.contains(&candidate.live);
            if newly_prioritized || retry_deferred {
                self.remaining.insert(candidate.live);
                if !scheduled.contains(&candidate.live) {
                    additions.push(candidate);
                }
            }
        }
        let round = self.round.as_ref().expect(ROUND_PRESENT_DETAIL);
        self.queues
            .enqueue_priority(&additions, round, &self.remaining)
    }
}

include!("pty_session_released_coverage_round_macos.rs");

#[cfg(any(test, feature = "runtime-component-tests"))]
include!("../../../../test_support/pty_session/pty_session_released_coverage_macos.rs");

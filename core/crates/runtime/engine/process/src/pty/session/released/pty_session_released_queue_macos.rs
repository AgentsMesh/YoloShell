use super::universe::ReleasedCandidate;
use crate::process::LiveProcessIdentity;
use crate::process::inventory_stable::{
    MAX_STABLE_INVENTORY_OBSERVATIONS, STABLE_INVENTORY_SNAPSHOTS_PER_CAPTURE,
};
use anyhow::{Result, ensure};
use std::collections::{HashMap, HashSet, VecDeque};

pub(super) const RELEASED_SWEEP_QUEUE_ENTRY_LIMIT: usize = MAX_STABLE_INVENTORY_OBSERVATIONS;
const QUEUE_LIMIT_DETAIL: &str = "released sweep queue exceeds the stable inventory bound";

const _: () = assert!(STABLE_INVENTORY_SNAPSHOTS_PER_CAPTURE > 0);

#[derive(Clone, Copy, Default, Eq, PartialEq)]
pub(super) enum SweepLane {
    #[default]
    Original,
    PriorityOld,
    PriorityNew,
}

impl SweepLane {
    fn next(self) -> Self {
        match self {
            Self::Original => Self::PriorityOld,
            Self::PriorityOld => Self::PriorityNew,
            Self::PriorityNew => Self::Original,
        }
    }
}

#[derive(Clone, Copy)]
pub(super) struct ScheduledCandidate {
    pub(super) candidate: ReleasedCandidate,
    lane: SweepLane,
}

pub(super) enum QueueTake {
    Candidate(ScheduledCandidate, usize),
    Exhausted(usize),
    Yielded(usize),
}

#[derive(Clone, Default)]
pub(super) struct ReleasedSweepQueues {
    original: VecDeque<ReleasedCandidate>,
    priority_old: VecDeque<ReleasedCandidate>,
    priority_new: VecDeque<ReleasedCandidate>,
    next_lane: SweepLane,
}

impl ReleasedSweepQueues {
    pub(super) fn start(&mut self, ordered: &[ReleasedCandidate]) -> Result<()> {
        ensure!(
            ordered.len() <= RELEASED_SWEEP_QUEUE_ENTRY_LIMIT,
            QUEUE_LIMIT_DETAIL
        );
        self.original = ordered.iter().copied().collect();
        self.priority_old.clear();
        self.priority_new.clear();
        self.next_lane = SweepLane::default();
        Ok(())
    }

    pub(super) fn clear(&mut self) {
        self.original.clear();
        self.priority_old.clear();
        self.priority_new.clear();
        self.next_lane = SweepLane::default();
    }

    pub(super) fn enqueue_priority(
        &mut self,
        additions: &[ReleasedCandidate],
        round: &HashMap<LiveProcessIdentity, ReleasedCandidate>,
        remaining: &HashSet<LiveProcessIdentity>,
    ) -> Result<()> {
        if additions.is_empty() {
            return Ok(());
        }
        self.priority_old.append(&mut self.priority_new);
        if self.len().saturating_add(additions.len()) > RELEASED_SWEEP_QUEUE_ENTRY_LIMIT {
            self.compact(round, remaining, additions);
        }
        ensure!(
            self.len().saturating_add(additions.len()) <= RELEASED_SWEEP_QUEUE_ENTRY_LIMIT,
            QUEUE_LIMIT_DETAIL
        );
        for candidate in additions.iter().copied() {
            self.priority_new.push_back(candidate);
        }
        Ok(())
    }

    pub(super) fn take_next(
        &mut self,
        round: &HashMap<LiveProcessIdentity, ReleasedCandidate>,
        remaining: &HashSet<LiveProcessIdentity>,
        work_limit: usize,
    ) -> QueueTake {
        let mut inspected = usize::default();
        let mut empty_cycle_start = None;
        while inspected < work_limit {
            let lane = self.next_lane;
            self.next_lane = lane.next();
            let Some(candidate) = self.pop(lane) else {
                let cycle_start = empty_cycle_start.get_or_insert(lane);
                if self.next_lane == *cycle_start {
                    return QueueTake::Exhausted(inspected);
                }
                continue;
            };
            empty_cycle_start = None;
            if remaining.contains(&candidate.live) && round.get(&candidate.live) == Some(&candidate)
            {
                inspected += 1;
                return QueueTake::Candidate(ScheduledCandidate { candidate, lane }, inspected);
            }
        }
        if self.is_empty() {
            QueueTake::Exhausted(inspected)
        } else {
            QueueTake::Yielded(inspected)
        }
    }

    pub(super) fn restore_retry(&mut self, scheduled: ScheduledCandidate) {
        match scheduled.lane {
            SweepLane::Original => self.original.push_back(scheduled.candidate),
            SweepLane::PriorityOld | SweepLane::PriorityNew => {
                self.priority_old.push_back(scheduled.candidate);
            }
        }
    }

    pub(super) fn len(&self) -> usize {
        self.original.len() + self.priority_old.len() + self.priority_new.len()
    }

    fn is_empty(&self) -> bool {
        self.original.is_empty() && self.priority_old.is_empty() && self.priority_new.is_empty()
    }

    fn pop(&mut self, lane: SweepLane) -> Option<ReleasedCandidate> {
        match lane {
            SweepLane::Original => self.original.pop_front(),
            SweepLane::PriorityOld => self.priority_old.pop_front(),
            SweepLane::PriorityNew => self.priority_new.pop_front(),
        }
    }

    fn compact(
        &mut self,
        round: &HashMap<LiveProcessIdentity, ReleasedCandidate>,
        remaining: &HashSet<LiveProcessIdentity>,
        additions: &[ReleasedCandidate],
    ) {
        let replaced = additions
            .iter()
            .map(|candidate| candidate.live)
            .collect::<HashSet<_>>();
        let mut retained = HashSet::new();
        let mut keep = |candidate: &ReleasedCandidate| {
            !replaced.contains(&candidate.live)
                && remaining.contains(&candidate.live)
                && round.get(&candidate.live) == Some(candidate)
                && retained.insert(candidate.live)
        };
        self.priority_old.retain(&mut keep);
        self.priority_new.retain(&mut keep);
        self.original.retain(keep);
    }
}

#[cfg(test)]
#[cfg(any(not(test), feature = "yolo-integration-tests"))]
include!("../../../../integration_tests/pty/session/released/pty_session_released_queue_macos_tests.rs");

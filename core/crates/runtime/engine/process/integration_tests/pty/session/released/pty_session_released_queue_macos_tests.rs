#[cfg(test)]
mod tests {
    use super::*;
    use crate::process::ProcessIdentity;

    const RETRY_PID: u32 = 141_001;
    const RETRY_GENERATION: u64 = 151_001;
    const MIDDLE_PID: u32 = 141_002;
    const MIDDLE_GENERATION: u64 = 151_002;
    const FIRST_FRESH_PID: u32 = 141_003;
    const FIRST_FRESH_GENERATION: u64 = 151_003;
    const FRESH_CANDIDATE_COUNT: usize = 4;
    const TEST_START_TIME: u64 = 161_001;
    const ONE_EXTRA_ENTRY: usize = 1;
    const SINGLE_CANDIDATE_WORK: usize = 1;
    const SINGLE_QUEUE_ENTRY: usize = 1;
    const COMPACTED_LIVE_ENTRY_COUNT: usize = 3;
    const COMPACTION_EXACT_PID: u32 = 141_004;
    const COMPACTION_AMBIENT_PID: u32 = 141_005;
    const COMPACTION_ADDITION_PID: u32 = 141_006;
    const COMPACTION_STALE_PID: u32 = 141_007;
    const COMPACTION_EXACT_GENERATION: u64 = 151_004;
    const COMPACTION_AMBIENT_GENERATION: u64 = 151_005;
    const COMPACTION_ADDITION_GENERATION: u64 = 151_006;
    const COMPACTION_STALE_GENERATION: u64 = 151_007;
    const CANDIDATE_SCHEDULE_DETAIL: &str = "candidate is scheduled";

    #[test]
    fn old_retry_and_fresh_work_cannot_starve_existing_priority() {
        let retry = candidate(RETRY_PID, RETRY_GENERATION);
        let middle = candidate(MIDDLE_PID, MIDDLE_GENERATION);
        let mut queues = ReleasedSweepQueues::default();
        let mut round = HashMap::from([(retry.live, retry)]);
        let mut remaining = HashSet::from([retry.live]);
        queues.start(&[]).unwrap();
        queues
            .enqueue_priority(&[retry], &round, &remaining)
            .unwrap();
        let first = take_candidate(&mut queues, &round, &remaining);
        assert_eq!(first.candidate, retry);
        queues.restore_retry(first);

        round.insert(middle.live, middle);
        remaining.insert(middle.live);
        queues
            .enqueue_priority(&[middle], &round, &remaining)
            .unwrap();
        let retried = take_candidate(&mut queues, &round, &remaining);
        assert_eq!(retried.candidate, retry);
        queues.restore_retry(retried);

        let mut middle_scheduled = false;
        for offset in usize::default()..FRESH_CANDIDATE_COUNT {
            let fresh = candidate(
                FIRST_FRESH_PID + u32::try_from(offset).unwrap(),
                FIRST_FRESH_GENERATION + u64::try_from(offset).unwrap(),
            );
            round.insert(fresh.live, fresh);
            remaining.insert(fresh.live);
            queues
                .enqueue_priority(&[fresh], &round, &remaining)
                .unwrap();
            let scheduled = take_candidate(&mut queues, &round, &remaining);
            if scheduled.candidate == retry {
                queues.restore_retry(scheduled);
            } else {
                remaining.remove(&scheduled.candidate.live);
            }
            middle_scheduled |= scheduled.candidate == middle;
        }
        assert!(middle_scheduled);
    }

    #[test]
    fn oversized_initial_universe_returns_an_error_instead_of_panicking() {
        let repeated = candidate(RETRY_PID, RETRY_GENERATION);
        let oversized = vec![repeated; RELEASED_SWEEP_QUEUE_ENTRY_LIMIT + ONE_EXTRA_ENTRY];
        let mut queues = ReleasedSweepQueues::default();

        assert!(queues.start(&oversized).is_err());
        assert!(queues.is_empty());
    }

    #[test]
    fn stale_tombstone_does_not_consume_live_candidate_work() {
        let stale = candidate(RETRY_PID, RETRY_GENERATION);
        let live = candidate(MIDDLE_PID, MIDDLE_GENERATION);
        let round = HashMap::from([(live.live, live)]);
        let remaining = HashSet::from([live.live]);
        let mut queues = ReleasedSweepQueues::default();
        queues.start(&[stale]).unwrap();
        queues
            .enqueue_priority(&[live], &round, &remaining)
            .unwrap();

        match queues.take_next(&round, &remaining, SINGLE_CANDIDATE_WORK) {
            QueueTake::Candidate(scheduled, consumed) => {
                assert_eq!(scheduled.candidate, live);
                assert_eq!(consumed, SINGLE_CANDIDATE_WORK);
            }
            QueueTake::Exhausted(_) | QueueTake::Yielded(_) => {
                panic!("{CANDIDATE_SCHEDULE_DETAIL}")
            }
        }
    }

    #[test]
    fn compaction_discards_a_full_stale_backlog_before_live_addition() {
        let stale = candidate(RETRY_PID, RETRY_GENERATION);
        let live = candidate(MIDDLE_PID, MIDDLE_GENERATION);
        let round = HashMap::from([(live.live, live)]);
        let remaining = HashSet::from([live.live]);
        let mut queues = ReleasedSweepQueues::default();
        queues
            .start(&vec![stale; RELEASED_SWEEP_QUEUE_ENTRY_LIMIT])
            .unwrap();

        queues
            .enqueue_priority(&[live], &round, &remaining)
            .unwrap();

        assert_eq!(queues.len(), SINGLE_QUEUE_ENTRY);
        assert_eq!(
            take_candidate(&mut queues, &round, &remaining).candidate,
            live
        );
    }

    #[test]
    fn compaction_preserves_an_exact_retry_priority_copy() {
        let exact = candidate(COMPACTION_EXACT_PID, COMPACTION_EXACT_GENERATION);
        let ambient = candidate(COMPACTION_AMBIENT_PID, COMPACTION_AMBIENT_GENERATION);
        let addition = candidate(COMPACTION_ADDITION_PID, COMPACTION_ADDITION_GENERATION);
        let stale = candidate(COMPACTION_STALE_PID, COMPACTION_STALE_GENERATION);
        let mut round = HashMap::from([(exact.live, exact), (ambient.live, ambient)]);
        let mut remaining = HashSet::from([exact.live, ambient.live]);
        let mut queues = ReleasedSweepQueues::default();
        queues.start(&[exact, ambient]).unwrap();
        queues
            .enqueue_priority(&[exact], &round, &remaining)
            .unwrap();

        let first = take_candidate(&mut queues, &round, &remaining);
        assert_eq!(first.candidate, exact);
        queues.restore_retry(first);
        let stale_entry_count = RELEASED_SWEEP_QUEUE_ENTRY_LIMIT.saturating_sub(queues.len());
        queues
            .original
            .extend(std::iter::repeat(stale).take(stale_entry_count));
        round.insert(addition.live, addition);
        remaining.insert(addition.live);

        queues
            .enqueue_priority(&[addition], &round, &remaining)
            .unwrap();

        assert_eq!(queues.len(), COMPACTED_LIVE_ENTRY_COUNT);
        assert_eq!(
            take_candidate(&mut queues, &round, &remaining).candidate,
            exact
        );
    }

    fn take_candidate(
        queues: &mut ReleasedSweepQueues,
        round: &HashMap<LiveProcessIdentity, ReleasedCandidate>,
        remaining: &HashSet<LiveProcessIdentity>,
    ) -> ScheduledCandidate {
        match queues.take_next(round, remaining, SINGLE_CANDIDATE_WORK) {
            QueueTake::Candidate(candidate, _) => candidate,
            QueueTake::Exhausted(_) | QueueTake::Yielded(_) => {
                panic!("{CANDIDATE_SCHEDULE_DETAIL}")
            }
        }
    }

    fn candidate(pid: u32, generation: u64) -> ReleasedCandidate {
        ReleasedCandidate {
            live: LiveProcessIdentity::from_kernel(pid, generation).unwrap(),
            durable: ProcessIdentity {
                pid,
                start_time: TEST_START_TIME,
            },
            parent_lineage: u64::default(),
            stable: false,
            capability_root: false,
            environment_eligible: true,
        }
    }
}

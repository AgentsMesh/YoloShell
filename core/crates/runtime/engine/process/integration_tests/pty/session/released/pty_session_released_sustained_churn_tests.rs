use super::super::super::boundary::RELEASED_LIVE_SWEEP_MINIMUM_WORK_PER_POLL;
use super::super::super::queue::RELEASED_SWEEP_QUEUE_ENTRY_LIMIT;
use super::*;
use std::collections::HashSet;

const CHURN_BATCH_MULTIPLIER: usize = 2;
const OVER_BUDGET_ENTRY_COUNT: usize = 1;
const CHURN_CANDIDATES_PER_POLL: usize =
    RELEASED_LIVE_SWEEP_MINIMUM_WORK_PER_POLL * CHURN_BATCH_MULTIPLIER + OVER_BUDGET_ENTRY_COUNT;
const POST_BOUNDARY_POLLS: usize = 2;
const CHURN_POLL_COUNT: usize =
    RELEASED_SWEEP_QUEUE_ENTRY_LIMIT.div_ceil(CHURN_CANDIDATES_PER_POLL) + POST_BOUNDARY_POLLS;
const FIRST_CHURN_ROUND: usize = 1;
const INITIAL_WORK_LIMIT: usize = 0;
const FIRST_CHURN_PID: u32 = 121_000;
const FIRST_CHURN_GENERATION: u64 = 131_000;

#[test]
fn sustained_over_budget_replacement_is_bounded_and_evaluates_live_work() {
    let mut state = ReleasedScanState::default();
    let initial = advance_scan(
        replacement_universe(usize::default()),
        &state,
        INITIAL_WORK_LIMIT,
        |_candidate, _budget| unreachable!(),
    )
    .unwrap();
    commit(&mut state, initial);
    let mut maximum_queue = state.queued_entries_for_test();

    for round in FIRST_CHURN_ROUND..=CHURN_POLL_COUNT {
        let current = replacement_universe(round);
        let live = current
            .iter()
            .map(|candidate| candidate.live)
            .collect::<HashSet<_>>();
        let mut evaluated = Vec::new();
        let result = advance_scan(
            current,
            &state,
            RELEASED_LIVE_SWEEP_MINIMUM_WORK_PER_POLL,
            |candidate, _budget| {
                evaluated.push(candidate.live);
                Ok(CandidateEvaluation::Complete(false))
            },
        )
        .unwrap();

        assert!(!evaluated.is_empty());
        assert!(evaluated.iter().all(|identity| live.contains(identity)));
        assert_eq!(result.candidates.status, CapabilityScanStatus::Pending);
        assert_eq!(result.coverage_status, CapabilityScanStatus::Pending);
        commit(&mut state, result);
        let queued = state.queued_entries_for_test();
        assert!(queued <= RELEASED_SWEEP_QUEUE_ENTRY_LIMIT);
        maximum_queue = maximum_queue.max(queued);
    }

    assert!(maximum_queue > CHURN_CANDIDATES_PER_POLL);
}

fn replacement_universe(round: usize) -> Arc<[ReleasedCandidate]> {
    let first = round * CHURN_CANDIDATES_PER_POLL;
    (first..first + CHURN_CANDIDATES_PER_POLL)
        .map(|offset| {
            candidate(
                FIRST_CHURN_PID + u32::try_from(offset).unwrap(),
                FIRST_CHURN_GENERATION + u64::try_from(offset).unwrap(),
                false,
            )
        })
        .collect::<Vec<_>>()
        .into()
}

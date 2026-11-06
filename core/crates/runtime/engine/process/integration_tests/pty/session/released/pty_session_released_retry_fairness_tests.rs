use super::*;

const EXPECTED_FIRST_ATTEMPTS: usize = 1;
const UNCONSUMED_DEFERRED_POLLS: usize = 3;

#[test]
fn perpetually_retried_original_head_cannot_starve_later_candidates() {
    let retry = candidate(THIRD_PID, THIRD_GENERATION, false);
    let matched = candidate(SECOND_PID, SECOND_GENERATION, true);
    let tail = candidate(FIRST_PID, FIRST_GENERATION, false);
    let current = candidates([tail, matched, retry]);
    let mut state = ReleasedScanState::default();
    let mut evaluated = Vec::new();

    for _poll in [retry, matched, tail] {
        let result = advance_scan(
            Arc::clone(&current),
            &state,
            ONE_CANDIDATE_PER_BATCH,
            |candidate, _budget| {
                evaluated.push(candidate);
                Ok(if candidate == retry {
                    CandidateEvaluation::Retry
                } else {
                    CandidateEvaluation::Complete(candidate == matched)
                })
            },
        )
        .unwrap();
        assert_eq!(result.coverage_status, CapabilityScanStatus::Pending);
        commit(&mut state, result);
    }

    assert_eq!(evaluated, [retry, matched, tail]);
    assert_eq!(
        evaluated
            .iter()
            .filter(|candidate| **candidate == retry)
            .count(),
        EXPECTED_FIRST_ATTEMPTS
    );
    assert!(state.covers_exact_closure(&[matched.live]));
    assert_eq!(state.pending_lineages_for_test(), [retry.live.lineage_id()]);
}

#[test]
fn deferred_exact_priority_retries_before_ambient_round_completion() {
    let exact = candidate(THIRD_PID, THIRD_GENERATION, false);
    let ambient = candidate(FIRST_PID, FIRST_GENERATION, false);
    let current = candidates([exact, ambient]);
    let priorities = HashSet::from([exact.live]);
    let mut state = ReleasedScanState::default();
    let mut evaluated = Vec::new();

    let first = advance::advance_with_priority_boundary(
        Arc::clone(&current),
        &state,
        boundary::ReleasedPollBoundary::work_only(ONE_CANDIDATE_PER_BATCH),
        ReleasedSweepPolicy::Frozen,
        &priorities,
        |candidate, _budget| {
            evaluated.push(candidate);
            Ok(CandidateEvaluation::Deferred)
        },
    )
    .unwrap();
    commit(&mut state, first);

    let second = advance::advance_with_priority_boundary(
        current,
        &state,
        boundary::ReleasedPollBoundary::work_only(ONE_CANDIDATE_PER_BATCH),
        ReleasedSweepPolicy::Frozen,
        &priorities,
        |candidate, _budget| {
            evaluated.push(candidate);
            Ok(CandidateEvaluation::Complete(candidate == exact))
        },
    )
    .unwrap();
    commit(&mut state, second);

    assert_eq!(evaluated, [exact, exact]);
    assert!(state.covers_exact_closure(&[exact.live]));
    assert_eq!(
        state.pending_lineages_for_test(),
        [ambient.live.lineage_id()]
    );
}

#[test]
fn queued_deferred_exact_priority_is_not_duplicated_before_consumption() {
    let exact = candidate(THIRD_PID, THIRD_GENERATION, false);
    let current = candidates([exact, candidate(FIRST_PID, FIRST_GENERATION, false)]);
    let priorities = HashSet::from([exact.live]);
    let mut state = ReleasedScanState::default();
    let deferred = advance::advance_with_priority_boundary(
        Arc::clone(&current),
        &state,
        boundary::ReleasedPollBoundary::work_only(ONE_CANDIDATE_PER_BATCH),
        ReleasedSweepPolicy::Frozen,
        &priorities,
        |_candidate, _budget| Ok(CandidateEvaluation::Deferred),
    )
    .unwrap();
    commit(&mut state, deferred);
    let queued_before = state.queued_entries_for_test();

    for _poll in 0..UNCONSUMED_DEFERRED_POLLS {
        let pending = advance::advance_with_priority_boundary(
            Arc::clone(&current),
            &state,
            boundary::ReleasedPollBoundary::work_only(usize::default()),
            ReleasedSweepPolicy::Frozen,
            &priorities,
            |_candidate, _budget| unreachable!(),
        )
        .unwrap();
        commit(&mut state, pending);
        assert_eq!(state.queued_entries_for_test(), queued_before + 1);
    }
}

include!("pty_session_released_positive_publication_tests.rs");

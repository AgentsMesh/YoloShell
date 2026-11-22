use super::*;

const RETRY_FAIRNESS_POLL_LIMIT: usize = 3;
const EXPECTED_EXACT_RETRY_ATTEMPTS: usize = 2;
const EXACT_POSITIVE_EVALUATED_AMBIENT_DETAIL: &str =
    "exact positive did not publish before ambient evaluation";

#[test]
fn exact_positive_publishes_and_retains_the_ambient_round() {
    let exact = candidate(THIRD_PID, THIRD_GENERATION, false);
    let ambient = candidate(FIRST_PID, FIRST_GENERATION, false);
    let current = candidates([ambient, exact]);
    let priorities = HashSet::from([exact.live]);
    let mut state = ReleasedScanState::default();
    let mut evaluated = Vec::new();

    let published = frozen_priority_poll(
        Arc::clone(&current),
        &state,
        MAX_PTY_PROOF_CANDIDATES,
        &priorities,
        |candidate, _budget| {
            evaluated.push(candidate);
            assert_eq!(
                candidate, exact,
                "{EXACT_POSITIVE_EVALUATED_AMBIENT_DETAIL}"
            );
            Ok(CandidateEvaluation::Complete(true))
        },
    );

    assert_eq!(evaluated, [exact]);
    assert_eq!(published.candidates.status, CapabilityScanStatus::Pending);
    assert_eq!(published.scheduling_status, CapabilityScanStatus::Pending);
    assert_eq!(published.coverage_status, CapabilityScanStatus::Pending);
    assert_eq!(
        published.candidates.observed,
        HashSet::from([exact.live.lineage_id()])
    );
    commit(&mut state, published);
    assert!(state.covers_exact_closure(&[exact.live]));
    assert_eq!(state.round_size_for_test(), current.len());
    assert_eq!(
        state.pending_lineages_for_test(),
        [ambient.live.lineage_id()]
    );

    let resumed = frozen_priority_poll(
        current,
        &state,
        MAX_PTY_PROOF_CANDIDATES,
        &priorities,
        |candidate, _budget| {
            evaluated.push(candidate);
            assert_eq!(candidate, ambient);
            Ok(CandidateEvaluation::Complete(false))
        },
    );

    assert_eq!(evaluated, [exact, ambient]);
    assert_eq!(resumed.scheduling_status, CapabilityScanStatus::Complete);
    assert_eq!(resumed.coverage_status, CapabilityScanStatus::Complete);
}

#[test]
fn non_exact_positive_and_exact_false_obey_the_full_quantum() {
    let exact = candidate(THIRD_PID, THIRD_GENERATION, false);
    let matched = candidate(SECOND_PID, SECOND_GENERATION, false);
    let tail = candidate(FIRST_PID, FIRST_GENERATION, false);
    let priorities = HashSet::from([exact.live]);
    let mut evaluated = Vec::new();

    let result = frozen_priority_poll(
        candidates([tail, matched, exact]),
        &ReleasedScanState::default(),
        MAX_PTY_PROOF_CANDIDATES,
        &priorities,
        |candidate, _budget| {
            evaluated.push(candidate);
            Ok(CandidateEvaluation::Complete(candidate == matched))
        },
    );

    assert_eq!(evaluated, [exact, matched, tail]);
    assert_eq!(result.scheduling_status, CapabilityScanStatus::Complete);
    assert_eq!(result.coverage_status, CapabilityScanStatus::Complete);
    assert_eq!(
        result.candidates.observed,
        HashSet::from([matched.live.lineage_id()])
    );
}

#[test]
fn exact_deferred_does_not_hide_ambient_work() {
    let exact = candidate(THIRD_PID, THIRD_GENERATION, false);
    let ambient = candidate(FIRST_PID, FIRST_GENERATION, false);
    let priorities = HashSet::from([exact.live]);
    let mut evaluated = Vec::new();

    let result = frozen_priority_poll(
        candidates([ambient, exact]),
        &ReleasedScanState::default(),
        MAX_PTY_PROOF_CANDIDATES,
        &priorities,
        |candidate, _budget| {
            evaluated.push(candidate);
            Ok(if candidate == exact {
                CandidateEvaluation::Deferred
            } else {
                CandidateEvaluation::Complete(false)
            })
        },
    );

    assert_eq!(evaluated, [exact, ambient]);
    assert_eq!(result.scheduling_status, CapabilityScanStatus::Complete);
    assert_eq!(result.coverage_status, CapabilityScanStatus::Pending);
}

#[test]
fn exact_retry_remains_bounded_and_preserves_ambient_fairness() {
    let exact = candidate(THIRD_PID, THIRD_GENERATION, false);
    let ambient = candidate(FIRST_PID, FIRST_GENERATION, false);
    let current = candidates([ambient, exact]);
    let priorities = HashSet::from([exact.live]);
    let mut state = ReleasedScanState::default();
    let mut evaluated = Vec::new();

    for _poll in 0..RETRY_FAIRNESS_POLL_LIMIT {
        let result = frozen_priority_poll(
            Arc::clone(&current),
            &state,
            ONE_CANDIDATE_PER_BATCH,
            &priorities,
            |candidate, _budget| {
                evaluated.push(candidate);
                Ok(if candidate == exact {
                    CandidateEvaluation::Retry
                } else {
                    CandidateEvaluation::Complete(false)
                })
            },
        );
        assert_eq!(result.scheduling_status, CapabilityScanStatus::Pending);
        assert_eq!(result.coverage_status, CapabilityScanStatus::Pending);
        commit(&mut state, result);
        if evaluated.contains(&ambient) {
            break;
        }
    }

    assert_eq!(evaluated, [exact, exact, ambient]);
    assert_eq!(
        evaluated
            .iter()
            .filter(|candidate| **candidate == exact)
            .count(),
        EXPECTED_EXACT_RETRY_ATTEMPTS
    );
    assert_eq!(state.pending_lineages_for_test(), [exact.live.lineage_id()]);
}

fn frozen_priority_poll<F>(
    current: Arc<[ReleasedCandidate]>,
    state: &ReleasedScanState,
    work_limit: usize,
    priorities: &HashSet<LiveProcessIdentity>,
    evaluate: F,
) -> CapabilityScanResult
where
    F: FnMut(ReleasedCandidate, &mut CapabilityScanBudget) -> anyhow::Result<CandidateEvaluation>,
{
    advance::advance_with_priority_boundary(
        current,
        state,
        boundary::ReleasedPollBoundary::work_only(work_limit),
        ReleasedSweepPolicy::Frozen,
        priorities,
        evaluate,
    )
    .unwrap()
}

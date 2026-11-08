use super::*;

#[test]
fn frozen_retry_rechecks_only_the_deferred_candidate() {
    let conclusive = candidate(FIRST_PID, FIRST_GENERATION, false);
    let deferred = candidate(SECOND_PID, SECOND_GENERATION, false);
    let current = candidates([conclusive, deferred]);
    let mut state = inconclusive_state(Arc::clone(&current), deferred);
    let mut evaluated = Vec::new();

    let retried = advance_with_policy(
        current,
        &state,
        MAX_PTY_PROOF_CANDIDATES,
        ReleasedSweepPolicy::Frozen,
        |candidate, _budget| {
            evaluated.push(candidate);
            Ok(CandidateEvaluation::Complete(false))
        },
    )
    .unwrap();

    assert_eq!(retried.coverage_status, CapabilityScanStatus::Complete);
    assert_eq!(evaluated, [deferred]);
    commit(&mut state, retried);
}

#[test]
fn frozen_retry_drops_a_vanished_deferred_candidate() {
    let conclusive = candidate(FIRST_PID, FIRST_GENERATION, false);
    let deferred = candidate(SECOND_PID, SECOND_GENERATION, false);
    let mut state = inconclusive_state(candidates([conclusive, deferred]), deferred);

    let retried = advance_with_policy(
        candidates([conclusive]),
        &state,
        MAX_PTY_PROOF_CANDIDATES,
        ReleasedSweepPolicy::Frozen,
        |_candidate, _budget| unreachable!(),
    )
    .unwrap();

    assert_eq!(retried.coverage_status, CapabilityScanStatus::Complete);
    commit(&mut state, retried);
}

#[test]
fn frozen_retry_defers_an_ambient_arrival_to_the_next_snapshot() {
    let conclusive = candidate(FIRST_PID, FIRST_GENERATION, false);
    let deferred = candidate(SECOND_PID, SECOND_GENERATION, false);
    let arrival = candidate(THIRD_PID, THIRD_GENERATION, false);
    let mut state = inconclusive_state(candidates([conclusive, deferred]), deferred);
    let mut evaluated = Vec::new();

    let retried = advance_with_policy(
        candidates([conclusive, deferred, arrival]),
        &state,
        MAX_PTY_PROOF_CANDIDATES,
        ReleasedSweepPolicy::Frozen,
        |candidate, _budget| {
            evaluated.push(candidate);
            Ok(CandidateEvaluation::Complete(false))
        },
    )
    .unwrap();

    assert_eq!(retried.coverage_status, CapabilityScanStatus::Complete);
    assert_eq!(evaluated, [deferred]);
    commit(&mut state, retried);
}

fn inconclusive_state(
    current: Arc<[ReleasedCandidate]>,
    deferred: ReleasedCandidate,
) -> ReleasedScanState {
    let mut state = ReleasedScanState::default();
    let result = advance_with_policy(
        current,
        &state,
        MAX_PTY_PROOF_CANDIDATES,
        ReleasedSweepPolicy::Frozen,
        |candidate, _budget| {
            Ok(if candidate == deferred {
                CandidateEvaluation::Deferred
            } else {
                CandidateEvaluation::Complete(false)
            })
        },
    )
    .unwrap();
    assert_eq!(result.scheduling_status, CapabilityScanStatus::Complete);
    assert_eq!(result.coverage_status, CapabilityScanStatus::Pending);
    commit(&mut state, result);
    state
}

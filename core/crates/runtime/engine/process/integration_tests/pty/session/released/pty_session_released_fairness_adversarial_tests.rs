use super::*;

const FRESH_PID_BASE: u32 = 42_001;
const FRESH_GENERATION_BASE: u64 = 52_001;
const FRESH_ARRIVAL_ATTEMPTS: usize = 6;
const EXPECTED_DEFERRED_ATTEMPTS: usize = 2;
const FIRST_EVALUATION_DETAIL: &str = "released sweep did not evaluate its initial head";
const MIDDLE_EVALUATION_DETAIL: &str = "fresh churn starved the middle backlog candidate";
const OLDEST_EVALUATION_DETAIL: &str = "fresh churn starved the oldest backlog candidate";

#[test]
fn repeated_fresh_arrivals_cannot_starve_the_old_backlog() {
    let oldest = candidate(FIRST_PID, FIRST_GENERATION, false);
    let middle = candidate(SECOND_PID, SECOND_GENERATION, false);
    let head = candidate(THIRD_PID, THIRD_GENERATION, false);
    let mut universe = vec![oldest, middle, head];
    let mut state = ReleasedScanState::default();
    let mut evaluated = Vec::new();
    complete_one(&universe, &mut state, &mut evaluated);

    for offset in usize::default()..FRESH_ARRIVAL_ATTEMPTS {
        universe.push(fresh_candidate(offset));
        complete_one(&universe, &mut state, &mut evaluated);
        if evaluated.contains(&oldest) {
            break;
        }
    }

    assert_eq!(evaluated.first(), Some(&head), "{FIRST_EVALUATION_DETAIL}");
    let middle_index = evaluated
        .iter()
        .position(|candidate| *candidate == middle)
        .expect(MIDDLE_EVALUATION_DETAIL);
    let oldest_index = evaluated
        .iter()
        .position(|candidate| *candidate == oldest)
        .expect(OLDEST_EVALUATION_DETAIL);
    assert!(middle_index < oldest_index);
}

#[test]
fn deferred_old_work_retries_after_the_churned_round_completes() {
    let deferred = candidate(THIRD_PID, THIRD_GENERATION, false);
    let middle = candidate(SECOND_PID, SECOND_GENERATION, false);
    let oldest = candidate(FIRST_PID, FIRST_GENERATION, false);
    let mut universe = vec![oldest, middle, deferred];
    let mut state = ReleasedScanState::default();
    let mut evaluated = Vec::new();
    defer_target_once(&universe, deferred, &mut state, &mut evaluated);

    universe.push(fresh_candidate(usize::default()));
    complete_one(&universe, &mut state, &mut evaluated);
    complete_one(&universe, &mut state, &mut evaluated);
    let completed = advance_scan(
        universe.clone().into(),
        &state,
        MAX_PTY_PROOF_CANDIDATES,
        |candidate, _budget| {
            evaluated.push(candidate);
            Ok(CandidateEvaluation::Complete(false))
        },
    )
    .unwrap();
    assert_eq!(completed.scheduling_status, CapabilityScanStatus::Complete);
    assert_eq!(completed.coverage_status, CapabilityScanStatus::Pending);
    commit(&mut state, completed);
    let retried = advance_scan(
        universe.clone().into(),
        &state,
        MAX_PTY_PROOF_CANDIDATES,
        |candidate, _budget| {
            evaluated.push(candidate);
            Ok(if candidate == deferred {
                CandidateEvaluation::Deferred
            } else {
                CandidateEvaluation::Complete(false)
            })
        },
    )
    .unwrap();

    assert_eq!(retried.scheduling_status, CapabilityScanStatus::Complete);
    assert_eq!(retried.coverage_status, CapabilityScanStatus::Pending);
    assert_eq!(retried.candidates.status, CapabilityScanStatus::Pending);
    assert_eq!(
        evaluated
            .iter()
            .filter(|candidate| **candidate == deferred)
            .count(),
        EXPECTED_DEFERRED_ATTEMPTS
    );
    assert!(evaluated.contains(&middle));
    assert!(evaluated.contains(&oldest));
}

#[test]
fn perpetually_deferred_head_cannot_starve_a_later_exact_match() {
    let deferred = candidate(THIRD_PID, THIRD_GENERATION, false);
    let matched = candidate(SECOND_PID, SECOND_GENERATION, true);
    let tail = candidate(FIRST_PID, FIRST_GENERATION, false);
    let mut state = ReleasedScanState::default();
    let mut evaluated = Vec::new();

    let result = advance_scan(
        candidates([tail, matched, deferred]),
        &state,
        MAX_PTY_PROOF_CANDIDATES,
        |candidate, _budget| {
            evaluated.push(candidate);
            Ok(if candidate == deferred {
                CandidateEvaluation::Deferred
            } else {
                CandidateEvaluation::Complete(candidate == matched)
            })
        },
    )
    .unwrap();

    assert_eq!(evaluated, [deferred, matched, tail]);
    assert_eq!(result.scheduling_status, CapabilityScanStatus::Complete);
    assert_eq!(result.coverage_status, CapabilityScanStatus::Pending);
    commit(&mut state, result);
    assert!(state.covers_exact_closure(&[matched.live]));
}

fn complete_one(
    universe: &[ReleasedCandidate],
    state: &mut ReleasedScanState,
    evaluated: &mut Vec<ReleasedCandidate>,
) {
    scan_one(universe, state, evaluated, |_| {
        CandidateEvaluation::Complete(false)
    });
}

fn defer_target_once(
    universe: &[ReleasedCandidate],
    deferred: ReleasedCandidate,
    state: &mut ReleasedScanState,
    evaluated: &mut Vec<ReleasedCandidate>,
) {
    scan_one(universe, state, evaluated, |candidate| {
        if candidate == deferred {
            CandidateEvaluation::Deferred
        } else {
            CandidateEvaluation::Complete(false)
        }
    });
}

fn scan_one(
    universe: &[ReleasedCandidate],
    state: &mut ReleasedScanState,
    evaluated: &mut Vec<ReleasedCandidate>,
    mut evaluate: impl FnMut(ReleasedCandidate) -> CandidateEvaluation,
) {
    let result = advance_scan(
        universe.to_vec().into(),
        state,
        ONE_CANDIDATE_PER_BATCH,
        |candidate, _budget| {
            evaluated.push(candidate);
            Ok(evaluate(candidate))
        },
    )
    .unwrap();
    commit(state, result);
}

fn fresh_candidate(offset: usize) -> ReleasedCandidate {
    candidate(
        FRESH_PID_BASE + u32::try_from(offset).unwrap(),
        FRESH_GENERATION_BASE + u64::try_from(offset).unwrap(),
        false,
    )
}

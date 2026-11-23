use super::*;
use crate::process::pty_session_proof::PtyCapabilityScanState;

const REPEATED_NEGATIVE_SWEEPS: usize = 4;
const COMPLETED_PREFIX_CANDIDATES: usize = 2;

#[test]
fn repeated_identical_negative_sweeps_never_publish_settlement() {
    let current = candidates([candidate(FIRST_PID, FIRST_GENERATION, false)]);
    let mut state = ReleasedScanState::default();

    for _sweep in 0..REPEATED_NEGATIVE_SWEEPS {
        let result = excluding(Arc::clone(&current), &state, ONE_CANDIDATE_PER_BATCH);
        assert_eq!(result.coverage_status, CapabilityScanStatus::Complete);
        assert_eq!(result.candidates.status, CapabilityScanStatus::Pending);
        commit(&mut state, result);
    }
}

#[test]
fn empty_released_sweeps_remain_fail_closed() {
    let result = excluding(
        Arc::from([]),
        &ReleasedScanState::default(),
        MAX_PTY_PROOF_CANDIDATES,
    );

    assert_eq!(result.scheduling_status, CapabilityScanStatus::Complete);
    assert_eq!(result.coverage_status, CapabilityScanStatus::Complete);
    assert_eq!(result.candidates.status, CapabilityScanStatus::Pending);
}

#[test]
fn universe_change_preserves_bounded_scheduling_progress() {
    let first = candidate(FIRST_PID, FIRST_GENERATION, false);
    let second = candidate(SECOND_PID, SECOND_GENERATION, false);
    let added = candidate(THIRD_PID, THIRD_GENERATION, false);
    let mut state = ReleasedScanState::default();
    let initial = excluding(candidates([first, second]), &state, ONE_CANDIDATE_PER_BATCH);
    commit(&mut state, initial);
    let mut evaluated = Vec::new();

    let restarted = advance_scan(
        candidates([first, second, added]),
        &state,
        ONE_CANDIDATE_PER_BATCH,
        |candidate, _budget| {
            evaluated.push(candidate);
            Ok(CandidateEvaluation::Complete(false))
        },
    )
    .unwrap();

    assert_eq!(evaluated, [added]);
    assert_eq!(restarted.coverage_status, CapabilityScanStatus::Pending);
    assert_eq!(restarted.candidates.status, CapabilityScanStatus::Pending);
}

#[test]
fn retry_neither_advances_scheduling_nor_publishes_settlement() {
    let only = candidate(FIRST_PID, FIRST_GENERATION, false);
    let current = candidates([only]);
    let mut state = ReleasedScanState::default();
    for _attempt in 0..REPEATED_NEGATIVE_SWEEPS {
        let result = advance_scan(
            Arc::clone(&current),
            &state,
            ONE_CANDIDATE_PER_BATCH,
            |_candidate, _budget| Ok(CandidateEvaluation::Retry),
        )
        .unwrap();
        assert_eq!(result.coverage_status, CapabilityScanStatus::Pending);
        assert_eq!(result.candidates.status, CapabilityScanStatus::Pending);
        commit(&mut state, result);
    }
    let completed = excluding(current, &state, ONE_CANDIDATE_PER_BATCH);
    assert_eq!(completed.coverage_status, CapabilityScanStatus::Complete);
    assert_eq!(completed.candidates.status, CapabilityScanStatus::Pending);
}

#[test]
fn deferred_candidate_completes_scheduling_without_settlement_authority() {
    let first = candidate(FIRST_PID, FIRST_GENERATION, false);
    let second = candidate(SECOND_PID, SECOND_GENERATION, false);
    let result = advance_scan(
        candidates([first, second]),
        &ReleasedScanState::default(),
        MAX_PTY_PROOF_CANDIDATES,
        |candidate, _budget| {
            Ok(if candidate == first {
                CandidateEvaluation::Deferred
            } else {
                CandidateEvaluation::Complete(false)
            })
        },
    )
    .unwrap();

    assert_eq!(result.scheduling_status, CapabilityScanStatus::Complete);
    assert_eq!(result.coverage_status, CapabilityScanStatus::Pending);
    assert_eq!(result.candidates.status, CapabilityScanStatus::Pending);
}

#[test]
fn scan_restart_preserves_matches_and_resets_scheduling() {
    let matched = candidate(THIRD_PID, THIRD_GENERATION, false);
    let completed = candidate(SECOND_PID, SECOND_GENERATION, false);
    let pending = candidate(FIRST_PID, FIRST_GENERATION, false);
    let current = candidates([matched, completed, pending]);
    let mut state = ReleasedScanState::default();
    let observed = advance_scan(
        Arc::clone(&current),
        &state,
        COMPLETED_PREFIX_CANDIDATES,
        |candidate, _budget| Ok(CandidateEvaluation::Complete(candidate == matched)),
    )
    .unwrap();
    commit(&mut state, observed);

    let mut restarted = PtyCapabilityScanState {
        released: state,
        ..PtyCapabilityScanState::default()
    };
    restarted.restart();
    assert!(restarted.released.matched.contains_key(&matched.live));
    let mut evaluated = Vec::new();
    let next = advance_scan(
        current,
        &restarted.released,
        COMPLETED_PREFIX_CANDIDATES,
        |candidate, _budget| {
            evaluated.push(candidate);
            Ok(CandidateEvaluation::Complete(false))
        },
    )
    .unwrap();
    assert_eq!(evaluated, [completed]);
    assert_eq!(next.coverage_status, CapabilityScanStatus::Pending);
    assert_eq!(next.candidates.status, CapabilityScanStatus::Pending);
}

#[test]
fn retained_match_covers_only_its_exact_nonempty_closure() {
    let matched = candidate(FIRST_PID, FIRST_GENERATION, false);
    let deferred = candidate(SECOND_PID, SECOND_GENERATION, false);
    let mut state = ReleasedScanState::default();
    let result = advance_scan(
        candidates([matched, deferred]),
        &state,
        MAX_PTY_PROOF_CANDIDATES,
        |candidate, _budget| {
            Ok(if candidate == matched {
                CandidateEvaluation::Complete(true)
            } else {
                CandidateEvaluation::Deferred
            })
        },
    )
    .unwrap();
    assert_eq!(result.scheduling_status, CapabilityScanStatus::Complete);
    assert_eq!(result.coverage_status, CapabilityScanStatus::Pending);
    commit(&mut state, result);

    assert!(state.covers_exact_closure(&[matched.live]));
    assert!(!state.covers_exact_closure(&[]));
    assert!(!state.covers_exact_closure(&[matched.live, deferred.live]));
}

fn excluding(
    current: Arc<[ReleasedCandidate]>,
    state: &ReleasedScanState,
    limit: usize,
) -> CapabilityScanResult {
    advance_scan(current, state, limit, |_candidate, _budget| {
        Ok(CandidateEvaluation::Complete(false))
    })
    .unwrap()
}

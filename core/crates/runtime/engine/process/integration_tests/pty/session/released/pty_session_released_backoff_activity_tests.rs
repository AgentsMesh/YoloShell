use super::*;
use crate::process::CapabilityScanActivity;

const REPEATED_NEGATIVE_ROUND_COUNT: usize = 3;
const ACTIVE_ROUND_EXACT_WORK_LIMIT: usize = ONE_CANDIDATE_PER_BATCH;

#[test]
fn split_negative_round_only_reports_new_snapshot_evidence_once() {
    let current = candidates([
        candidate(FIRST_PID, FIRST_GENERATION, false),
        candidate(SECOND_PID, SECOND_GENERATION, false),
        candidate(THIRD_PID, THIRD_GENERATION, false),
    ]);
    let mut state = ReleasedScanState::default();

    let initial = negative_batch(Arc::clone(&current), &state, ONE_CANDIDATE_PER_BATCH);
    assert_eq!(activity(&initial), CapabilityScanActivity::Evidence);
    assert_eq!(initial.scheduling_status, CapabilityScanStatus::Pending);
    assert!(initial.advance.batch_consumed);
    commit_state(&mut state, initial);

    let middle = negative_batch(Arc::clone(&current), &state, ONE_CANDIDATE_PER_BATCH);
    assert_eq!(activity(&middle), CapabilityScanActivity::ContinueRound);
    assert_eq!(middle.scheduling_status, CapabilityScanStatus::Pending);
    assert!(middle.advance.batch_consumed);
    commit_state(&mut state, middle);

    let completed = negative_batch(current, &state, ONE_CANDIDATE_PER_BATCH);
    assert_eq!(activity(&completed), CapabilityScanActivity::Idle);
    assert_eq!(completed.scheduling_status, CapabilityScanStatus::Complete);
    assert!(completed.advance.batch_consumed);
}

#[test]
fn identical_complete_negative_rounds_do_not_publish_repeated_evidence() {
    let current = candidates([candidate(FIRST_PID, FIRST_GENERATION, false)]);
    let mut state = ReleasedScanState::default();
    let initial = negative_batch(Arc::clone(&current), &state, MAX_PTY_PROOF_CANDIDATES);
    assert_eq!(activity(&initial), CapabilityScanActivity::Evidence);
    commit_state(&mut state, initial);

    for _ in usize::default()..REPEATED_NEGATIVE_ROUND_COUNT {
        let repeated = negative_batch(Arc::clone(&current), &state, MAX_PTY_PROOF_CANDIDATES);
        assert_eq!(activity(&repeated), CapabilityScanActivity::Idle);
        assert_eq!(repeated.scheduling_status, CapabilityScanStatus::Complete);
        assert!(repeated.advance.batch_consumed);
        commit_state(&mut state, repeated);
    }
}

#[test]
fn repeated_deferred_round_backs_off_until_conclusive_evidence_arrives() {
    let current = candidates([candidate(FIRST_PID, FIRST_GENERATION, false)]);
    let mut state = ReleasedScanState::default();
    let initial = deferred_batch(Arc::clone(&current), &state);
    assert_eq!(activity(&initial), CapabilityScanActivity::Evidence);
    assert_eq!(initial.coverage_status, CapabilityScanStatus::Pending);
    commit_state(&mut state, initial);

    let repeated = deferred_batch(Arc::clone(&current), &state);
    assert_eq!(activity(&repeated), CapabilityScanActivity::Idle);
    assert_eq!(repeated.coverage_status, CapabilityScanStatus::Pending);
    assert!(repeated.advance.batch_consumed);
    commit_state(&mut state, repeated);

    let conclusive = negative_batch(current, &state, MAX_PTY_PROOF_CANDIDATES);
    assert_eq!(activity(&conclusive), CapabilityScanActivity::Evidence);
    assert_eq!(conclusive.coverage_status, CapabilityScanStatus::Complete);
}

#[test]
fn active_round_deferred_then_conclusive_negative_reports_evidence() {
    let exact = candidate(THIRD_PID, THIRD_GENERATION, false);
    let ambient = candidate(FIRST_PID, FIRST_GENERATION, false);
    let current = candidates([ambient, exact]);
    let priorities = HashSet::from([exact.live]);
    let mut state = ReleasedScanState::default();

    let deferred = advance::advance_with_priority_boundary(
        Arc::clone(&current),
        &state,
        boundary::ReleasedPollBoundary::work_only(ACTIVE_ROUND_EXACT_WORK_LIMIT),
        ReleasedSweepPolicy::Frozen,
        &priorities,
        |candidate, _budget| {
            assert_eq!(candidate, exact);
            Ok(CandidateEvaluation::Deferred)
        },
    )
    .unwrap();
    assert_eq!(activity(&deferred), CapabilityScanActivity::Evidence);
    assert_eq!(deferred.coverage_status, CapabilityScanStatus::Pending);
    commit_state(&mut state, deferred);

    let mut evaluated = Vec::new();
    let conclusive = advance::advance_with_priority_boundary(
        current,
        &state,
        boundary::ReleasedPollBoundary::work_only(MAX_PTY_PROOF_CANDIDATES),
        ReleasedSweepPolicy::Frozen,
        &priorities,
        |candidate, _budget| {
            evaluated.push(candidate);
            Ok(CandidateEvaluation::Complete(false))
        },
    )
    .unwrap();

    assert_eq!(evaluated, [exact, ambient]);
    assert_eq!(conclusive.scheduling_status, CapabilityScanStatus::Complete);
    assert_eq!(conclusive.coverage_status, CapabilityScanStatus::Complete);
    assert_eq!(activity(&conclusive), CapabilityScanActivity::Evidence);
}

fn negative_batch(
    current: Arc<[ReleasedCandidate]>,
    state: &ReleasedScanState,
    work_limit: usize,
) -> CapabilityScanResult {
    advance_scan(current, state, work_limit, |_candidate, _budget| {
        Ok(CandidateEvaluation::Complete(false))
    })
    .unwrap()
}

fn deferred_batch(
    current: Arc<[ReleasedCandidate]>,
    state: &ReleasedScanState,
) -> CapabilityScanResult {
    advance_scan(
        current,
        state,
        MAX_PTY_PROOF_CANDIDATES,
        |_candidate, _budget| Ok(CandidateEvaluation::Deferred),
    )
    .unwrap()
}

fn activity(result: &CapabilityScanResult) -> CapabilityScanActivity {
    result.advance.activity(result.scheduling_status)
}

fn commit_state(state: &mut ReleasedScanState, result: CapabilityScanResult) {
    *state = result.advance.next.released;
}

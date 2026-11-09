use super::*;

const EXPECTED_CONSUMED_EVALUATION_COUNT: usize = 2;
#[test]
fn frozen_round_defers_new_arrivals_to_the_next_snapshot() {
    let first = candidate(FIRST_PID, FIRST_GENERATION, false);
    let second = candidate(SECOND_PID, SECOND_GENERATION, false);
    let added = candidate(THIRD_PID, THIRD_GENERATION, false);
    let mut state = ReleasedScanState::default();
    let initial = excluding_frozen(candidates([first, second]), &state, ONE_CANDIDATE_PER_BATCH);
    commit(&mut state, initial);
    let mut evaluated = Vec::new();

    let completed = advance_with_policy(
        candidates([first, second, added]),
        &state,
        MAX_PTY_PROOF_CANDIDATES,
        ReleasedSweepPolicy::Frozen,
        |candidate, _budget| {
            evaluated.push(candidate);
            Ok(CandidateEvaluation::Complete(false))
        },
    )
    .unwrap();

    assert_eq!(completed.coverage_status, CapabilityScanStatus::Complete);
    assert!(!evaluated.contains(&added));
    commit(&mut state, completed);

    let next = advance_with_policy(
        candidates([first, second, added]),
        &state,
        MAX_PTY_PROOF_CANDIDATES,
        ReleasedSweepPolicy::Frozen,
        |candidate, _budget| {
            evaluated.push(candidate);
            Ok(CandidateEvaluation::Complete(false))
        },
    )
    .unwrap();
    assert_eq!(next.coverage_status, CapabilityScanStatus::Complete);
    assert!(evaluated.contains(&added));
}

#[test]
fn frozen_round_admits_a_new_exact_anchor_without_admitting_ambient_churn() {
    const AMBIENT_PID: u32 = 81_001;
    const AMBIENT_GENERATION: u64 = 91_001;
    let first = candidate(FIRST_PID, FIRST_GENERATION, false);
    let second = candidate(SECOND_PID, SECOND_GENERATION, false);
    let exact = candidate(THIRD_PID, THIRD_GENERATION, false);
    let ambient = candidate(AMBIENT_PID, AMBIENT_GENERATION, false);
    let mut state = ReleasedScanState::default();
    let initial = excluding_frozen(candidates([first, second]), &state, ONE_CANDIDATE_PER_BATCH);
    commit(&mut state, initial);
    let mut evaluated = Vec::new();

    let promoted = advance::advance_with_priority_boundary(
        candidates([first, second, exact, ambient]),
        &state,
        boundary::ReleasedPollBoundary::work_only(ONE_CANDIDATE_PER_BATCH),
        ReleasedSweepPolicy::Frozen,
        &HashSet::from([exact.live]),
        |candidate, _budget| {
            evaluated.push(candidate);
            Ok(CandidateEvaluation::Complete(false))
        },
    )
    .unwrap();
    commit(&mut state, promoted);

    assert_eq!(evaluated, [exact]);
    assert!(
        !state
            .pending_lineages_for_test()
            .contains(&ambient.live.lineage_id())
    );
}

#[test]
fn frozen_round_rechecks_a_consumed_candidate_each_time_priority_reappears() {
    let pending = candidate(FIRST_PID, FIRST_GENERATION, false);
    let consumed = candidate(SECOND_PID, SECOND_GENERATION, false);
    let current = candidates([pending, consumed]);
    let mut state = ReleasedScanState::default();
    let initial = excluding_frozen(Arc::clone(&current), &state, ONE_CANDIDATE_PER_BATCH);
    commit(&mut state, initial);
    let mut evaluated = Vec::new();

    let promoted = advance::advance_with_priority_boundary(
        Arc::clone(&current),
        &state,
        boundary::ReleasedPollBoundary::work_only(ONE_CANDIDATE_PER_BATCH),
        ReleasedSweepPolicy::Frozen,
        &HashSet::from([consumed.live]),
        |candidate, _budget| {
            evaluated.push(candidate);
            Ok(CandidateEvaluation::Complete(false))
        },
    )
    .unwrap();
    assert_eq!(promoted.scheduling_status, CapabilityScanStatus::Pending);
    commit(&mut state, promoted);

    let demoted = advance::advance_with_priority_boundary(
        Arc::clone(&current),
        &state,
        boundary::ReleasedPollBoundary::work_only(usize::default()),
        ReleasedSweepPolicy::Frozen,
        &HashSet::new(),
        |_candidate, _budget| unreachable!(),
    )
    .unwrap();
    commit(&mut state, demoted);

    let promoted_again = advance::advance_with_priority_boundary(
        current,
        &state,
        boundary::ReleasedPollBoundary::work_only(MAX_PTY_PROOF_CANDIDATES),
        ReleasedSweepPolicy::Frozen,
        &HashSet::from([consumed.live]),
        |candidate, _budget| {
            evaluated.push(candidate);
            Ok(CandidateEvaluation::Complete(false))
        },
    )
    .unwrap();

    assert_eq!(
        evaluated
            .iter()
            .filter(|candidate| **candidate == consumed)
            .count(),
        EXPECTED_CONSUMED_EVALUATION_COUNT
    );
    assert_eq!(evaluated.last(), Some(&consumed));
    assert_eq!(
        promoted_again.scheduling_status,
        CapabilityScanStatus::Complete
    );
}

#[test]
fn frozen_round_drops_a_vanished_deferred_identity() {
    let survivor = candidate(FIRST_PID, FIRST_GENERATION, false);
    let vanished = candidate(SECOND_PID, SECOND_GENERATION, false);
    let mut state = ReleasedScanState::default();
    let deferred = advance_with_policy(
        candidates([survivor, vanished]),
        &state,
        ONE_CANDIDATE_PER_BATCH,
        ReleasedSweepPolicy::Frozen,
        |_candidate, _budget| Ok(CandidateEvaluation::Deferred),
    )
    .unwrap();
    commit(&mut state, deferred);

    let completed = excluding_frozen(candidates([survivor]), &state, MAX_PTY_PROOF_CANDIDATES);

    assert_eq!(completed.coverage_status, CapabilityScanStatus::Complete);
}

#[test]
fn frozen_deferred_identity_completes_scheduling_but_retries_for_coverage() {
    let candidate = candidate(FIRST_PID, FIRST_GENERATION, false);
    let current = candidates([candidate]);
    let mut state = ReleasedScanState::default();
    let deferred = advance_with_policy(
        Arc::clone(&current),
        &state,
        ONE_CANDIDATE_PER_BATCH,
        ReleasedSweepPolicy::Frozen,
        |_candidate, _budget| Ok(CandidateEvaluation::Deferred),
    )
    .unwrap();
    assert_eq!(deferred.scheduling_status, CapabilityScanStatus::Complete);
    assert_eq!(deferred.coverage_status, CapabilityScanStatus::Pending);
    commit(&mut state, deferred);

    let retried = excluding_frozen(current, &state, ONE_CANDIDATE_PER_BATCH);

    assert_eq!(retried.scheduling_status, CapabilityScanStatus::Complete);
    assert_eq!(retried.coverage_status, CapabilityScanStatus::Complete);
}

fn excluding_frozen(
    current: Arc<[ReleasedCandidate]>,
    state: &ReleasedScanState,
    limit: usize,
) -> CapabilityScanResult {
    advance_with_policy(
        current,
        state,
        limit,
        ReleasedSweepPolicy::Frozen,
        |_candidate, _budget| Ok(CandidateEvaluation::Complete(false)),
    )
    .unwrap()
}

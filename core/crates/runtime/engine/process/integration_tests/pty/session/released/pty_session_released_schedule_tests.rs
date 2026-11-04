use super::super::super::macos_candidate::CandidateEvaluation;
use super::super::advance::advance_with_priority_boundary;
use super::super::boundary::ReleasedPollBoundary;
use super::super::coverage::ReleasedSweepPolicy;
use super::*;
use crate::process::LiveProcessIdentity;
use std::collections::HashSet;
use std::sync::Arc;

const FIRST_PID: u32 = 61_001;
const SECOND_PID: u32 = 61_002;
const THIRD_PID: u32 = 61_003;
const FOURTH_PID: u32 = 61_004;
const FIRST_GENERATION: u64 = 71_001;
const SECOND_GENERATION: u64 = 71_002;
const THIRD_GENERATION: u64 = 71_003;
const TEST_START_TIME: u64 = 71_004;
const START_TIME_STEP: u64 = 1;

#[test]
fn empty_schedule_has_no_candidates() {
    assert!(ordered_schedule(&[], &HashSet::new()).is_empty());
}

#[test]
fn unordered_causal_input_is_sorted_by_exact_identity() {
    let first = candidate(FIRST_PID, FIRST_GENERATION);
    let second = candidate(SECOND_PID, SECOND_GENERATION);
    let third = candidate(THIRD_PID, THIRD_GENERATION);

    assert_eq!(
        ordered_schedule(&[third, first, second], &HashSet::new()),
        [third, second, first]
    );
}

#[test]
fn replacement_generations_have_a_stable_order() {
    let original = candidate(SECOND_PID, FIRST_GENERATION);
    let replacement = candidate(SECOND_PID, THIRD_GENERATION);
    let tail = candidate(THIRD_PID, SECOND_GENERATION);

    assert_eq!(
        ordered_schedule(&[tail, replacement, original], &HashSet::new()),
        [tail, original, replacement]
    );
}

#[test]
fn recent_external_candidate_precedes_older_external_candidate() {
    let recent = candidate(THIRD_PID, THIRD_GENERATION);
    let older = ReleasedCandidate {
        durable: crate::process::ProcessIdentity {
            start_time: TEST_START_TIME - START_TIME_STEP,
            ..candidate(FIRST_PID, FIRST_GENERATION).durable
        },
        environment_eligible: false,
        ..candidate(FIRST_PID, FIRST_GENERATION)
    };
    let recent = ReleasedCandidate {
        environment_eligible: false,
        ..recent
    };

    assert_eq!(
        ordered_schedule(&[older, recent], &HashSet::new()),
        [recent, older]
    );
}

#[test]
fn causal_candidates_precede_external_candidates_at_a_fresh_boundary() {
    let causal = candidate(THIRD_PID, THIRD_GENERATION);
    let external = ReleasedCandidate {
        environment_eligible: false,
        ..candidate(FIRST_PID, FIRST_GENERATION)
    };

    assert_eq!(
        ordered_schedule(&[external, causal], &HashSet::new()),
        [causal, external]
    );
}

#[test]
fn external_candidates_receive_bounded_service_among_causal_candidates() {
    let newest_causal = candidate(FOURTH_PID, THIRD_GENERATION);
    let older_causal = candidate(SECOND_PID, SECOND_GENERATION);
    let external = ReleasedCandidate {
        environment_eligible: false,
        ..candidate(THIRD_PID, FIRST_GENERATION)
    };

    assert_eq!(
        ordered_schedule(&[older_causal, external, newest_causal], &HashSet::new()),
        [newest_causal, external, older_causal]
    );
}

#[test]
fn exact_priority_is_fenced_to_the_captured_generation() {
    let exact = candidate(FIRST_PID, FIRST_GENERATION);
    let newer = candidate(THIRD_PID, THIRD_GENERATION);
    let replacement = LiveProcessIdentity::from_kernel(FIRST_PID, SECOND_GENERATION).unwrap();

    assert_eq!(
        ordered_schedule(&[exact, newer], &HashSet::from([exact.live])),
        [exact, newer]
    );
    assert_eq!(
        ordered_schedule(&[exact, newer], &HashSet::from([replacement])),
        [newer, exact]
    );
}

#[test]
fn mid_round_exact_priority_is_bounded_and_preserves_backlog_fairness() {
    const POLL_COUNT: usize = 3;
    const WORK_PER_POLL: usize = 1;
    let target = candidate(FIRST_PID, FIRST_GENERATION);
    let middle = candidate(SECOND_PID, SECOND_GENERATION);
    let head = candidate(THIRD_PID, THIRD_GENERATION);
    let universe: Arc<[ReleasedCandidate]> = Arc::from([target, middle, head]);
    let mut state = super::super::ReleasedScanState::default();
    let mut evaluated = Vec::new();
    let first = advance_with_priority_boundary(
        universe.clone(),
        &state,
        ReleasedPollBoundary::work_only(WORK_PER_POLL),
        ReleasedSweepPolicy::Frozen,
        &HashSet::new(),
        |candidate, _| {
            evaluated.push(candidate);
            Ok(CandidateEvaluation::Complete(false))
        },
    )
    .unwrap();
    state = first.advance.next.released;

    let exact = HashSet::from([target.live]);
    for _ in usize::default()..POLL_COUNT {
        let before = evaluated.len();
        let result = advance_with_priority_boundary(
            universe.clone(),
            &state,
            ReleasedPollBoundary::work_only(WORK_PER_POLL),
            ReleasedSweepPolicy::Frozen,
            &exact,
            |candidate, _| {
                evaluated.push(candidate);
                Ok(if candidate == target {
                    CandidateEvaluation::Retry
                } else {
                    CandidateEvaluation::Complete(false)
                })
            },
        )
        .unwrap();
        assert_eq!(evaluated.len() - before, WORK_PER_POLL);
        state = result.advance.next.released;
    }

    assert_eq!(evaluated, [head, target, middle, target]);
}

fn candidate(pid: u32, unique_id: u64) -> ReleasedCandidate {
    ReleasedCandidate {
        live: LiveProcessIdentity::from_kernel(pid, unique_id).unwrap(),
        durable: crate::process::ProcessIdentity {
            pid,
            start_time: TEST_START_TIME,
        },
        parent_lineage: u64::default(),
        stable: false,
        capability_root: false,
        environment_eligible: true,
    }
}

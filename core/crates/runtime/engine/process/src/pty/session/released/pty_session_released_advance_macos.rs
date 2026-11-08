use super::super::macos_candidate::CandidateEvaluation;
use super::boundary::ReleasedPollBoundary;
use super::coverage::ReleasedSweepPolicy;
use super::queue::QueueTake;
use super::result::{completed_round, materialize_evidence, pending, record_match};
use super::schedule::ordered_schedule;
use super::universe::ReleasedCandidate;
use super::{super::macos_contract::CapabilityScanBudget, ReleasedScanState};
use crate::process::LiveProcessIdentity;
use anyhow::Result;
use std::collections::HashSet;
use std::sync::Arc;

const SCHEDULED_CANDIDATE_ENTRY_COUNT: usize = 1;
type EvaluationResult = Result<CandidateEvaluation>;

#[cfg(all(test, feature = "yolo-integration-tests"))]
include!("../../../../integration_tests/pty/session/released/pty_session_released_advance_test_api_macos.rs");

pub(super) fn advance_with_holder_lifecycle(
    current: Arc<[ReleasedCandidate]>,
    state: &ReleasedScanState,
    boundary: ReleasedPollBoundary,
    policy: ReleasedSweepPolicy,
    exact_priorities: &HashSet<LiveProcessIdentity>,
    conclusive_holder_priorities: &HashSet<LiveProcessIdentity>,
    mut evaluate: impl FnMut(ReleasedCandidate, &mut CapabilityScanBudget) -> EvaluationResult,
) -> Result<super::CapabilityScanResult> {
    #[cfg(all(feature = "adversarial-test-hooks", not(test)))]
    super::super::macos_scan_diagnostic::record_poll(
        policy == ReleasedSweepPolicy::Frozen,
        state.coverage.has_active_frozen_round(),
    );
    let mut next = state.reconcile_current(&current);
    let ordered = ordered_schedule(&current, exact_priorities);
    let (mut scheduling_complete, mut coverage_complete, mut batch_consumed, mut evidence_observed) =
        next.coverage
            .begin_round(&ordered, policy, exact_priorities)?;
    let mut evidence = materialize_evidence(&next.matched);
    let mut budget = CapabilityScanBudget::start_batch();
    let mut consumed_work = usize::default();
    let boundary = boundary.begin_candidate_work();
    loop {
        if boundary.time_expired(consumed_work) {
            return Ok(pending(
                evidence,
                next,
                scheduling_complete,
                coverage_complete,
                batch_consumed,
                evidence_observed,
            ));
        }
        let taken = next
            .coverage
            .take_next(boundary.remaining_work(consumed_work));
        let (scheduled, inspected) = match taken {
            QueueTake::Candidate(scheduled, inspected) => (scheduled, inspected),
            QueueTake::Exhausted(inspected) => {
                batch_consumed |= inspected != usize::default();
                break;
            }
            QueueTake::Yielded(inspected) => {
                batch_consumed |= inspected != usize::default();
                return Ok(pending(
                    evidence,
                    next,
                    scheduling_complete,
                    coverage_complete,
                    batch_consumed,
                    evidence_observed,
                ));
            }
        };
        consumed_work += inspected;
        batch_consumed |= inspected > SCHEDULED_CANDIDATE_ENTRY_COUNT;
        let candidate = scheduled.candidate;
        if next.matched.contains_key(&candidate.live) {
            batch_consumed = true;
            let (attempt_scheduling_complete, attempt_coverage_complete) =
                next.coverage.record_attempt(scheduled, true);
            scheduling_complete |= attempt_scheduling_complete;
            coverage_complete |= attempt_coverage_complete;
            continue;
        }
        #[cfg(all(feature = "adversarial-test-hooks", not(test)))]
        let evaluation_started_at = std::time::Instant::now();
        let evaluation = evaluate(candidate, &mut budget)?;
        #[cfg(all(feature = "adversarial-test-hooks", not(test)))]
        super::super::macos_scan_diagnostic::record_evaluation(
            policy == ReleasedSweepPolicy::Frozen,
            evaluation,
            evaluation_started_at.elapsed(),
        );
        match evaluation {
            CandidateEvaluation::Retry => {
                next.coverage.restore_retry(scheduled);
                return Ok(pending(
                    evidence,
                    next,
                    scheduling_complete,
                    coverage_complete,
                    batch_consumed,
                    evidence_observed,
                ));
            }
            CandidateEvaluation::Deferred => {
                batch_consumed = true;
                let (attempt_scheduling_complete, attempt_coverage_complete) =
                    next.coverage.record_attempt(scheduled, false);
                scheduling_complete |= attempt_scheduling_complete;
                coverage_complete |= attempt_coverage_complete;
            }
            CandidateEvaluation::Complete(matches) => {
                batch_consumed = true;
                evidence_observed |= next.coverage.conclusive_observation_is_new(&candidate.live);
                let (attempt_scheduling_complete, attempt_coverage_complete) =
                    next.coverage.record_attempt(scheduled, true);
                scheduling_complete |= attempt_scheduling_complete;
                coverage_complete |= attempt_coverage_complete;
                if conclusive_holder_priorities.contains(&candidate.live) {
                    next.holder_hints.remove(&candidate.live);
                }
                if matches {
                    evidence_observed |= next.matched.insert(candidate.live, candidate).is_none();
                    record_match(&mut evidence, candidate);
                    if exact_priorities.contains(&candidate.live) {
                        return Ok(pending(
                            evidence,
                            next,
                            scheduling_complete,
                            coverage_complete,
                            batch_consumed,
                            evidence_observed,
                        ));
                    }
                }
            }
        }
    }
    Ok(completed_round(
        evidence,
        next,
        scheduling_complete,
        coverage_complete,
        batch_consumed,
        evidence_observed,
    ))
}

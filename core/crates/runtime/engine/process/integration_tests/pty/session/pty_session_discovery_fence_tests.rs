use super::*;
use crate::process::pty_session_proof::macos_contract::CapabilityProofError;
use anyhow::anyhow;

const QUERY_FAILURE_DETAIL: &str = "injected discovery candidate query failure";

#[test]
fn global_scan_defers_every_candidate_generation_loss() {
    assert_eq!(
        fenced_global_candidate(|| Ok(false), || Ok(true)).unwrap(),
        CandidateEvaluation::Deferred
    );
    assert_eq!(
        fenced_global_candidate(sequence(&[true, false]), || Ok(true)).unwrap(),
        CandidateEvaluation::Deferred
    );
    assert_eq!(
        fenced_global_candidate(sequence(&[true, false]), || {
            Err(CapabilityProofError::candidate(anyhow!(
                QUERY_FAILURE_DETAIL
            )))
        })
        .unwrap(),
        CandidateEvaluation::Deferred
    );
}

#[test]
fn global_scan_defers_candidate_local_failures_without_losing_invariants() {
    let deferred = fenced_global_candidate(
        || Ok(true),
        || {
            Err(CapabilityProofError::candidate(anyhow!(
                QUERY_FAILURE_DETAIL
            )))
        },
    )
    .unwrap();
    assert_eq!(deferred, CandidateEvaluation::Deferred);

    let invariant = fenced_global_candidate(
        || Ok(true),
        || {
            Err(CapabilityProofError::invariant(anyhow!(
                QUERY_FAILURE_DETAIL
            )))
        },
    )
    .unwrap_err();
    assert!(invariant.to_string().contains(QUERY_FAILURE_DETAIL));
}

#[test]
fn global_scan_defers_an_oversized_candidate_without_losing_coverage_progress() {
    let outcome = fenced_global_candidate(
        || Ok(true),
        || Err(CapabilityProofError::budget(anyhow!(QUERY_FAILURE_DETAIL))),
    )
    .unwrap();

    assert_eq!(outcome, CandidateEvaluation::Deferred);
}

#[test]
fn global_scan_retries_an_active_descriptor_spawn_boundary() {
    let outcome = fenced_global_candidate(
        || Ok(true),
        || Err(CapabilityProofError::descriptor_scan_deferred()),
    )
    .unwrap();

    assert_eq!(outcome, CandidateEvaluation::Retry);
}

#[test]
fn global_scan_retries_after_aggregate_batch_exhaustion() {
    let outcome = fenced_global_candidate(
        || Ok(true),
        || {
            Err(CapabilityProofError::BatchBudgetExhausted(anyhow!(
                QUERY_FAILURE_DETAIL
            )))
        },
    )
    .unwrap();

    assert_eq!(outcome, CandidateEvaluation::Retry);
}

#[test]
fn global_scan_defers_candidate_generation_query_failures() {
    let outcome = fenced_global_candidate(|| Err(anyhow!(QUERY_FAILURE_DETAIL)), || Ok(true));

    assert_eq!(outcome.unwrap(), CandidateEvaluation::Deferred);
}

fn sequence(values: &[bool]) -> impl FnMut() -> anyhow::Result<bool> + '_ {
    let mut values = values.iter().copied();
    move || Ok(values.next().unwrap())
}

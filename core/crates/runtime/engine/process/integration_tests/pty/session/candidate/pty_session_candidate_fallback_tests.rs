use super::*;
use anyhow::anyhow;
use std::cell::Cell;

const ENVIRONMENT_FAILURE_DETAIL: &str = "injected environment query failure";
const DESCRIPTOR_FAILURE_DETAIL: &str = "injected descriptor query failure";
const BATCH_FAILURE_DETAIL: &str = "injected batch exhaustion";
const INITIAL_GENERATION_CHECKS: usize = 0;
const GENERATION_CHECK_INCREMENT: usize = 1;
const ONE_GENERATION_CHECK: usize = 1;
const TWO_GENERATION_CHECKS: usize = 2;

#[test]
fn unreadable_environment_without_target_tty_is_safely_excluded() {
    let matched =
        match_tty_after_environment(local_error(ENVIRONMENT_FAILURE_DETAIL), || Ok(false)).unwrap();

    assert!(!matched);
}

#[test]
fn unreadable_environment_with_target_tty_remains_indeterminate() {
    let error = match_tty_after_environment(local_error(ENVIRONMENT_FAILURE_DETAIL), || Ok(true))
        .unwrap_err();

    assert!(error.to_string().contains(ENVIRONMENT_FAILURE_DETAIL));
    assert!(error.to_string().contains(CAPABILITY_QUERY_FAILURE_CONTEXT));
}

#[test]
fn two_local_query_failures_preserve_both_diagnostics() {
    let error = match_tty_after_environment(local_error(ENVIRONMENT_FAILURE_DETAIL), || {
        local_bool_error(DESCRIPTOR_FAILURE_DETAIL)
    })
    .unwrap_err();

    let detail = error.to_string();
    assert!(detail.contains(ENVIRONMENT_FAILURE_DETAIL));
    assert!(detail.contains(DESCRIPTOR_FAILURE_DETAIL));
    assert!(detail.contains(DESCRIPTOR_QUERY_FAILURE_CONTEXT));
}

#[test]
fn batch_exhaustion_defers_without_a_second_generation_query() {
    let checks = Cell::new(INITIAL_GENERATION_CHECKS);
    let outcome = fenced_candidate(
        || {
            checks.set(checks.get() + GENERATION_CHECK_INCREMENT);
            Ok(true)
        },
        || {
            Err(CapabilityProofError::BatchBudgetExhausted(anyhow!(
                BATCH_FAILURE_DETAIL
            )))
        },
    )
    .unwrap();

    assert_eq!(outcome, CandidateEvaluation::Retry);
    assert_eq!(checks.get(), ONE_GENERATION_CHECK);
}

#[test]
fn descriptor_exclusion_is_fenced_on_both_sides() {
    let checks = Cell::new(INITIAL_GENERATION_CHECKS);
    let outcome = fenced_candidate(
        || {
            checks.set(checks.get() + GENERATION_CHECK_INCREMENT);
            Ok(true)
        },
        || match_tty_after_environment(local_error(ENVIRONMENT_FAILURE_DETAIL), || Ok(false)),
    )
    .unwrap();

    assert_eq!(outcome, CandidateEvaluation::Complete(false));
    assert_eq!(checks.get(), TWO_GENERATION_CHECKS);
}

fn local_error(detail: &'static str) -> CapabilityProofResult<EnvironmentCapability> {
    Err(CapabilityProofError::candidate(anyhow!(detail)))
}

fn local_bool_error(detail: &'static str) -> CapabilityProofResult<bool> {
    Err(CapabilityProofError::candidate(anyhow!(detail)))
}

use super::*;
use crate::process::constants::{
    MAX_PTY_PROOF_CANDIDATES, MAX_PTY_PROOF_ENVIRONMENT_BYTES,
    MAX_PTY_PROOF_TOTAL_ENVIRONMENT_BYTES, MAX_PTY_PROOF_TOTAL_FILE_DESCRIPTORS,
};
use std::cell::Cell;

const BUDGET_FAILURE_DETAIL: &str = "injected PTY proof budget failure";
const INVARIANT_FAILURE_DETAIL: &str = "injected PTY proof invariant failure";
const BATCH_FAILURE_DETAIL: &str = "injected PTY proof batch exhaustion";
const OVER_BUDGET_INCREMENT: usize = 1;
const TEST_ENVIRONMENT_BUFFER_BYTES: usize = 256;
const INITIAL_GENERATION_CHECK: u8 = 0;
const GENERATION_CHECK_INCREMENT: u8 = 1;
const EXPECTED_UNFENCED_CHECKS: u8 = 1;

#[test]
fn aggregate_scan_budgets_fail_closed_at_the_named_limits() {
    assert!(CapabilityScanBudget::start(MAX_PTY_PROOF_CANDIDATES).is_ok());
    assert!(CapabilityScanBudget::start(MAX_PTY_PROOF_CANDIDATES + OVER_BUDGET_INCREMENT).is_err());

    let mut budget = CapabilityScanBudget::start(SINGLE_TEST_CANDIDATE).unwrap();
    budget
        .admit_descriptors(MAX_PTY_PROOF_TOTAL_FILE_DESCRIPTORS)
        .unwrap();
    assert!(budget.admit_descriptors(OVER_BUDGET_INCREMENT).is_err());

    let mut budget = CapabilityScanBudget::start(SINGLE_TEST_CANDIDATE).unwrap();
    for _ in 0..(MAX_PTY_PROOF_TOTAL_ENVIRONMENT_BYTES / MAX_PTY_PROOF_ENVIRONMENT_BYTES) {
        budget
            .admit_environment(MAX_PTY_PROOF_ENVIRONMENT_BYTES)
            .unwrap();
    }
    assert!(budget.admit_environment(OVER_BUDGET_INCREMENT).is_err());
    let mut budget = CapabilityScanBudget::start(SINGLE_TEST_CANDIDATE).unwrap();
    assert!(
        budget
            .admit_environment(MAX_PTY_PROOF_ENVIRONMENT_BYTES + OVER_BUDGET_INCREMENT)
            .is_err()
    );
}

#[test]
fn environment_capacity_is_bounded_without_charging_unused_bytes() {
    let budget = CapabilityScanBudget::start(SINGLE_TEST_CANDIDATE).unwrap();

    budget
        .validate_environment_capacity(MAX_PTY_PROOF_ENVIRONMENT_BYTES)
        .unwrap();

    assert_eq!(budget.environment_bytes(), usize::default());
    assert!(
        budget
            .validate_environment_capacity(MAX_PTY_PROOF_ENVIRONMENT_BYTES + OVER_BUDGET_INCREMENT,)
            .is_err()
    );
}

#[test]
fn environment_capture_buffer_is_reused_and_rejects_capacity_changes() {
    let mut budget = CapabilityScanBudget::start(SINGLE_TEST_CANDIDATE).unwrap();
    let first = budget
        .environment_capture_buffer(TEST_ENVIRONMENT_BUFFER_BYTES)
        .unwrap()
        .as_ptr();
    let second = budget
        .environment_capture_buffer(TEST_ENVIRONMENT_BUFFER_BYTES)
        .unwrap()
        .as_ptr();

    assert_eq!(first, second);
    assert_eq!(
        budget.cached_environment_capacity(),
        Some(TEST_ENVIRONMENT_BUFFER_BYTES)
    );
    assert!(
        budget
            .environment_capture_buffer(TEST_ENVIRONMENT_BUFFER_BYTES + OVER_BUDGET_INCREMENT)
            .is_err()
    );
}

#[test]
fn budget_error_is_not_omitted_when_the_candidate_disappears() {
    assert_non_local_error_is_not_omitted(
        CapabilityProofError::budget(anyhow::anyhow!(BUDGET_FAILURE_DETAIL)),
        BUDGET_FAILURE_DETAIL,
    );
}

#[test]
fn invariant_error_is_not_omitted_when_the_candidate_disappears() {
    assert_non_local_error_is_not_omitted(
        CapabilityProofError::invariant(anyhow::anyhow!(INVARIANT_FAILURE_DETAIL)),
        INVARIANT_FAILURE_DETAIL,
    );
}

#[test]
fn batched_exhaustion_defers_without_a_second_generation_check() {
    let checks = Cell::new(INITIAL_GENERATION_CHECK);
    let outcome = fenced_candidate(
        || {
            checks.set(checks.get() + GENERATION_CHECK_INCREMENT);
            Ok(true)
        },
        || {
            Err(CapabilityProofError::BatchBudgetExhausted(anyhow::anyhow!(
                BATCH_FAILURE_DETAIL
            )))
        },
    )
    .unwrap();

    assert_eq!(outcome, CandidateEvaluation::Retry);
    assert_eq!(checks.get(), EXPECTED_UNFENCED_CHECKS);
}

fn assert_non_local_error_is_not_omitted(error: CapabilityProofError, detail: &str) {
    let checks = Cell::new(INITIAL_GENERATION_CHECK);
    let error = candidate_matches(
        || {
            let check = checks.get();
            checks.set(check + GENERATION_CHECK_INCREMENT);
            Ok(check == INITIAL_GENERATION_CHECK)
        },
        || Err(error),
    )
    .unwrap_err();

    assert!(error.to_string().contains(detail));
    assert_eq!(checks.get(), EXPECTED_UNFENCED_CHECKS);
}

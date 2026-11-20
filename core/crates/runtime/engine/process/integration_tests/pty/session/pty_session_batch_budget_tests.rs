use super::*;
use crate::process::constants::{
    MAX_PTY_PROOF_ENVIRONMENT_BYTES, MAX_PTY_PROOF_TOTAL_ENVIRONMENT_BYTES,
    MAX_PTY_PROOF_TOTAL_FILE_DESCRIPTORS,
};

const OVER_LIMIT_INCREMENT: usize = 1;
const ERROR_CONTEXT_PID: u32 = 42_001;
const ERROR_CONTEXT_GENERATION: u64 = 42_002;
const ERROR_VARIANT_DETAIL: &str = "injected capability proof failure";
const EXPECTED_ERROR_VARIANTS: usize = 6;

#[test]
fn batched_aggregate_descriptor_limit_defers_instead_of_failing() {
    let mut budget = CapabilityScanBudget::start_batch();
    budget
        .admit_descriptors(MAX_PTY_PROOF_TOTAL_FILE_DESCRIPTORS)
        .unwrap();

    assert!(matches!(
        budget.admit_descriptors(OVER_LIMIT_INCREMENT),
        Err(CapabilityProofError::BatchBudgetExhausted(_))
    ));
}

#[test]
fn batched_aggregate_environment_limit_defers_instead_of_failing() {
    let mut budget = CapabilityScanBudget::start_batch();
    let candidates = MAX_PTY_PROOF_TOTAL_ENVIRONMENT_BYTES / MAX_PTY_PROOF_ENVIRONMENT_BYTES;
    for _ in 0..candidates {
        budget
            .admit_environment(MAX_PTY_PROOF_ENVIRONMENT_BYTES)
            .unwrap();
    }

    assert!(matches!(
        budget.admit_environment(OVER_LIMIT_INCREMENT),
        Err(CapabilityProofError::BatchBudgetExhausted(_))
    ));
}

#[test]
fn per_candidate_environment_limit_remains_a_hard_failure() {
    let mut budget = CapabilityScanBudget::start_batch();
    let over_limit = MAX_PTY_PROOF_ENVIRONMENT_BYTES + OVER_LIMIT_INCREMENT;

    assert!(matches!(
        budget.admit_environment(over_limit),
        Err(CapabilityProofError::ScanBudgetExceeded(_))
    ));
}

#[test]
fn atomic_aggregate_limit_remains_a_hard_failure() {
    let mut budget = CapabilityScanBudget::start(usize::default()).unwrap();
    budget
        .admit_descriptors(MAX_PTY_PROOF_TOTAL_FILE_DESCRIPTORS)
        .unwrap();

    assert!(matches!(
        budget.admit_descriptors(OVER_LIMIT_INCREMENT),
        Err(CapabilityProofError::ScanBudgetExceeded(_))
    ));
}

#[test]
fn descriptor_accounting_rejects_integer_overflow_in_both_modes() {
    let mut atomic = CapabilityScanBudget::start(usize::default()).unwrap();
    atomic.admit_descriptors(OVER_LIMIT_INCREMENT).unwrap();
    assert!(matches!(
        atomic.admit_descriptors(usize::MAX),
        Err(CapabilityProofError::ScanBudgetExceeded(_))
    ));

    let mut batched = CapabilityScanBudget::start_batch();
    batched.admit_descriptors(OVER_LIMIT_INCREMENT).unwrap();
    assert!(matches!(
        batched.admit_descriptors(usize::MAX),
        Err(CapabilityProofError::BatchBudgetExhausted(_))
    ));
}

#[test]
fn every_error_variant_preserves_identity_display_and_source_context() {
    let errors = [
        (
            CapabilityProofError::candidate(anyhow::anyhow!(ERROR_VARIANT_DETAIL)),
            ERROR_VARIANT_DETAIL,
        ),
        (
            CapabilityProofError::descriptor_scan_deferred(),
            DESCRIPTOR_SCAN_DEFERRED_DETAIL,
        ),
        (
            CapabilityProofError::holder_query(anyhow::anyhow!(ERROR_VARIANT_DETAIL)),
            ERROR_VARIANT_DETAIL,
        ),
        (
            CapabilityProofError::budget(anyhow::anyhow!(ERROR_VARIANT_DETAIL)),
            ERROR_VARIANT_DETAIL,
        ),
        (
            CapabilityProofError::batch(anyhow::anyhow!(ERROR_VARIANT_DETAIL)),
            ERROR_VARIANT_DETAIL,
        ),
        (
            CapabilityProofError::invariant(anyhow::anyhow!(ERROR_VARIANT_DETAIL)),
            ERROR_VARIANT_DETAIL,
        ),
    ];
    assert_eq!(errors.len(), EXPECTED_ERROR_VARIANTS);

    for (error, detail) in errors {
        let error = error.with_candidate_identity(ERROR_CONTEXT_PID, ERROR_CONTEXT_GENERATION);
        let displayed = error.to_string();
        assert!(displayed.contains(detail));
        assert!(displayed.contains(&ERROR_CONTEXT_PID.to_string()));
        assert!(displayed.contains(&ERROR_CONTEXT_GENERATION.to_string()));
        assert!(std::error::Error::source(&error).is_some());
    }
}

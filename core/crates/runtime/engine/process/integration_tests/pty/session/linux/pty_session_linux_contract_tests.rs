use super::*;

const SMALL_REMAINDER: usize = 7;

#[test]
fn fresh_batch_exposes_per_candidate_windows() {
    let budget = BatchBudget::new();

    assert_eq!(
        budget.environment_window().limit,
        LINUX_PTY_PROOF_ENVIRONMENT_BYTES
    );
    assert_eq!(
        budget.environment_window().exhausted,
        BudgetKind::CandidateEnvironment
    );
    assert_eq!(
        budget.descriptor_window().limit,
        LINUX_PTY_PROOF_FILE_DESCRIPTORS
    );
    assert_eq!(
        budget.descriptor_window().exhausted,
        BudgetKind::CandidateDescriptors
    );
}

#[test]
fn aggregate_exhaustion_is_distinct_from_candidate_exhaustion() {
    let mut budget = BatchBudget {
        environment: SMALL_REMAINDER,
        descriptors: SMALL_REMAINDER,
    };

    assert_eq!(budget.environment_window().limit, SMALL_REMAINDER);
    assert_eq!(
        budget.environment_window().exhausted,
        BudgetKind::BatchEnvironment
    );
    assert_eq!(budget.descriptor_window().limit, SMALL_REMAINDER);
    assert_eq!(
        budget.descriptor_window().exhausted,
        BudgetKind::BatchDescriptors
    );
    budget.charge_environment(usize::MAX);
    budget.charge_descriptors(usize::MAX);
    assert_eq!(budget.environment_window().limit, usize::default());
    assert_eq!(budget.descriptor_window().limit, usize::default());
}

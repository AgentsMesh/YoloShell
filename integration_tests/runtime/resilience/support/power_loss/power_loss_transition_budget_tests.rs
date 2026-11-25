use super::*;

const SINGLE_TREE: usize = 1;
const REBOOT_MATRIX_TREES: usize = 4;

#[test]
fn serial_freeze_preserves_each_participants_existing_budget() {
    assert_eq!(
        transition_budget(SINGLE_TREE).unwrap(),
        PROCESS_EXIT_TIMEOUT
    );
    let matrix = transition_budget(REBOOT_MATRIX_TREES).unwrap();
    let expected =
        (0..REBOOT_MATRIX_TREES).fold(Duration::ZERO, |sum, _| sum + PROCESS_EXIT_TIMEOUT);
    assert_eq!(matrix, expected);
}

#[test]
fn an_empty_batch_has_no_wait_budget_and_unrepresentable_counts_fail() {
    assert_eq!(transition_budget(usize::default()).unwrap(), Duration::ZERO);
    assert!(transition_budget(usize::MAX).is_err());
}

#[test]
fn transition_deadlines_are_bounded_by_the_exact_batch_budget() {
    let before = tokio::time::Instant::now();
    let deadline = transition_deadline(REBOOT_MATRIX_TREES).unwrap();
    let after = tokio::time::Instant::now();
    let budget = transition_budget(REBOOT_MATRIX_TREES).unwrap();
    assert!(deadline >= before + budget);
    assert!(deadline <= after + budget);
    assert!(transition_deadline(usize::MAX).is_err());
}

use super::*;

const SOFT_LIMIT: usize = 1_024;
const OPEN_DESCRIPTORS: usize = 100;
const RETAINED_AUTHORITIES: usize = 20;

#[test]
fn authority_capacity_accounts_for_open_and_retained_descriptors() {
    let expected = RETAINED_AUTHORITIES + SOFT_LIMIT
        - OPEN_DESCRIPTORS
        - LINUX_PTY_PROOF_RETAINED_FD_RESERVE
        - LINUX_PTY_PROOF_TRANSIENT_FD_BUDGET;
    let expected = expected.min(LINUX_PTY_PROOF_RETAINED_AUTHORITIES);

    assert_eq!(
        authority_capacity(SOFT_LIMIT, OPEN_DESCRIPTORS, RETAINED_AUTHORITIES),
        Some(expected)
    );
}

#[test]
fn exhausted_soft_limit_never_borrows_the_reserve() {
    let open = SOFT_LIMIT - LINUX_PTY_PROOF_RETAINED_FD_RESERVE;

    assert_eq!(
        authority_capacity(SOFT_LIMIT, open, usize::default()),
        Some(usize::default())
    );
}

#[test]
fn overflowed_baseline_is_indeterminate() {
    assert_eq!(
        authority_capacity(usize::MAX, usize::MAX, usize::default()),
        None
    );
}

#[test]
fn large_universe_cannot_expand_retained_validation_work() {
    assert_eq!(
        authority_capacity(usize::MAX, usize::default(), usize::default()),
        Some(LINUX_PTY_PROOF_RETAINED_AUTHORITIES)
    );
}

use super::*;
use std::cell::Cell;

const QUERY_CONTEXT: &str = "injected PTY proof query";
const GENERATION_CONTEXT: &str = "injected PTY generation query";
const INITIAL_GENERATION_CHECK: u8 = 0;
const GENERATION_CHECK_INCREMENT: u8 = 1;
const EXPECTED_FENCED_CHECKS: u8 = 2;

#[test]
fn einval_candidate_exit_is_omitted_after_a_generation_mismatch() {
    let checks = Cell::new(INITIAL_GENERATION_CHECK);
    let matched = candidate_matches(
        || {
            let check = checks.get();
            checks.set(check + GENERATION_CHECK_INCREMENT);
            Ok(check == INITIAL_GENERATION_CHECK)
        },
        || Err(candidate_query_error(libc::EINVAL)),
    )
    .unwrap();

    assert!(!matched);
    assert_eq!(checks.get(), EXPECTED_FENCED_CHECKS);
}

#[test]
fn candidate_error_for_the_same_generation_fails_closed() {
    let checks = Cell::new(INITIAL_GENERATION_CHECK);
    let error = candidate_matches(
        || {
            checks.set(checks.get() + GENERATION_CHECK_INCREMENT);
            Ok(true)
        },
        || Err(candidate_query_error(libc::EPERM)),
    )
    .unwrap_err();

    assert_eq!(raw_os_error(&error), Some(libc::EPERM));
    assert_eq!(checks.get(), EXPECTED_FENCED_CHECKS);
}

#[test]
fn indeterminate_error_fence_propagates_instead_of_omitting() {
    let checks = Cell::new(INITIAL_GENERATION_CHECK);
    let error = candidate_matches(
        || {
            let check = checks.get();
            checks.set(check + GENERATION_CHECK_INCREMENT);
            if check == INITIAL_GENERATION_CHECK {
                Ok(true)
            } else {
                Err(generation_error(libc::EIO))
            }
        },
        || Err(candidate_query_error(libc::EINVAL)),
    )
    .unwrap_err();

    assert_eq!(raw_os_error(&error), Some(libc::EIO));
    assert!(
        error
            .to_string()
            .contains(CANDIDATE_GENERATION_FENCE_CONTEXT)
    );
    assert_eq!(checks.get(), EXPECTED_FENCED_CHECKS);
}

#[test]
fn generation_change_after_a_successful_query_omits_the_candidate() {
    let checks = Cell::new(INITIAL_GENERATION_CHECK);
    let matched = candidate_matches(
        || {
            let check = checks.get();
            checks.set(check + GENERATION_CHECK_INCREMENT);
            Ok(check == INITIAL_GENERATION_CHECK)
        },
        || Ok(true),
    )
    .unwrap();

    assert!(!matched);
    assert_eq!(checks.get(), EXPECTED_FENCED_CHECKS);
}

#[test]
fn stale_generation_is_rejected_before_running_the_proof_query() {
    let queried = Cell::new(false);
    let matched = candidate_matches(
        || Ok(false),
        || {
            queried.set(true);
            Ok(true)
        },
    )
    .unwrap();

    assert!(!matched);
    assert!(!queried.get());
}

#[test]
fn candidate_without_target_tty_never_queries_its_environment() {
    let queried = Cell::new(false);
    let matched = match_environment_if_tty(false, || {
        queried.set(true);
        Err(candidate_query_error(libc::EPERM))
    })
    .unwrap();

    assert!(!matched);
    assert!(!queried.get());
}

#[test]
fn target_tty_candidate_environment_error_fails_closed() {
    let queried = Cell::new(false);
    let error = match_environment_if_tty(true, || {
        queried.set(true);
        Err(candidate_query_error(libc::EINVAL))
    })
    .unwrap_err();

    assert!(queried.get());
    assert_eq!(capability_raw_os_error(&error), Some(libc::EINVAL));
}

#[test]
fn missing_capability_never_queries_the_tty_descriptor() {
    let queried = Cell::new(false);
    let matched = match_tty_if_capability(EnvironmentCapability { matches: false }, || {
        queried.set(true);
        Err(candidate_query_error(libc::EPERM))
    })
    .unwrap();

    assert!(!matched);
    assert!(!queried.get());
}

#[test]
fn matching_capability_requires_the_tty_descriptor() {
    let queried = Cell::new(false);
    let matched = match_tty_if_capability(EnvironmentCapability { matches: true }, || {
        queried.set(true);
        Ok(true)
    })
    .unwrap();

    assert!(matched);
    assert!(queried.get());
}

fn candidate_query_error(errno: i32) -> CapabilityProofError {
    CapabilityProofError::candidate(
        anyhow::Error::new(std::io::Error::from_raw_os_error(errno)).context(QUERY_CONTEXT),
    )
}

fn generation_error(errno: i32) -> anyhow::Error {
    anyhow::Error::new(std::io::Error::from_raw_os_error(errno)).context(GENERATION_CONTEXT)
}

fn raw_os_error(error: &anyhow::Error) -> Option<i32> {
    error.chain().find_map(|cause| {
        cause
            .downcast_ref::<std::io::Error>()
            .and_then(std::io::Error::raw_os_error)
    })
}

fn capability_raw_os_error(error: &CapabilityProofError) -> Option<i32> {
    let error = match error {
        CapabilityProofError::CandidateLocalQuery(error)
        | CapabilityProofError::DescriptorScanDeferred(error)
        | CapabilityProofError::HolderInventoryQuery(error)
        | CapabilityProofError::ScanBudgetExceeded(error)
        | CapabilityProofError::BatchBudgetExhausted(error)
        | CapabilityProofError::Invariant(error) => error,
    };
    raw_os_error(error)
}

use super::ProcessClosureVersion;
use super::version::PROCESS_CLOSURE_VERSION_EXHAUSTED_DETAIL;

const EXPECTED_VERSION_ADVANCE_DETAIL: &str = "process closure version did not advance";
const EXPECTED_VERSION_EXHAUSTION_DETAIL: &str = "maximum process closure version advanced";
const CLONED_VERSION_CHANGED_DETAIL: &str = "cloned process closure version changed authority";

#[test]
fn closure_version_advances_without_reusing_its_predecessor() {
    let initial = ProcessClosureVersion::default();
    let next = initial.next().unwrap();

    assert_ne!(initial, next, "{EXPECTED_VERSION_ADVANCE_DETAIL}");
}

#[test]
fn closure_version_exhaustion_fails_without_wrapping() {
    let error = ProcessClosureVersion::maximum_for_test()
        .next()
        .expect_err(EXPECTED_VERSION_EXHAUSTION_DETAIL);

    assert_eq!(error.to_string(), PROCESS_CLOSURE_VERSION_EXHAUSTED_DETAIL);
}

#[test]
fn equal_generations_from_distinct_trees_have_distinct_authority() {
    let first = ProcessClosureVersion::default().next().unwrap();
    let second = ProcessClosureVersion::default().next().unwrap();

    assert_ne!(first, second, "{EXPECTED_VERSION_ADVANCE_DETAIL}");
}

#[test]
fn cloned_version_retains_the_exact_allocation_authority() {
    let version = ProcessClosureVersion::default().next().unwrap();

    assert_eq!(version, version.clone(), "{CLONED_VERSION_CHANGED_DETAIL}");
}

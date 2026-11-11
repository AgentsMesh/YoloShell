use super::*;

const PRIMARY_FAILURE: &str = "injected projection primary failure";
const INVALIDATION_FAILURE: &str = "injected projection invalidation failure";

#[cfg(feature = "yolo-unit-tests")]
#[test]
fn unexpected_success_becomes_a_primary_failure() {
    let error = primary_failure(Ok(Ok(())));
    assert_eq!(error.to_string(), PROJECTION_TASK_STOPPED);
}

#[cfg(feature = "yolo-unit-tests")]
#[test]
fn task_error_remains_the_primary_failure() {
    let error = primary_failure(Ok(Err(anyhow::anyhow!(PRIMARY_FAILURE))));
    assert_eq!(error.to_string(), PRIMARY_FAILURE);
}

#[cfg(feature = "yolo-unit-tests")]
#[test]
fn combined_failure_retains_primary_and_invalidation_details() {
    let primary = anyhow::anyhow!(PRIMARY_FAILURE);
    let invalidation = anyhow::anyhow!(INVALIDATION_FAILURE);
    let error = combine(primary, Err(invalidation)).unwrap_err();
    let detail = format!("{error:#}");
    assert!(detail.contains(PRIMARY_FAILURE));
    assert!(detail.contains(INVALIDATION_FAILURE));
}

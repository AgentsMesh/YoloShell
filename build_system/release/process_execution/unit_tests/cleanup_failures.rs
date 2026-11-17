use super::*;
use crate::unit_support::*;
use std::os::unix::process::ExitStatusExt;

#[test]
fn cleanup_preserves_each_failure_in_observation_order() {
    let mut failures = Failures::default();
    assert_eq!(
        failures.capture(Ok(EMPTY_WAIT_STATUS), FIRST_CONTEXT),
        Some(EMPTY_WAIT_STATUS)
    );
    failures.record(Ok(()), SECOND_CONTEXT);
    failures.record(Err(anyhow!(FAILURE_DETAIL)), FIRST_CONTEXT);
    failures.push(SECOND_CONTEXT);
    let status = ExitStatus::from_raw(EMPTY_WAIT_STATUS);
    let error = failures.finish(Some(status)).unwrap_err().to_string();
    assert_eq!(
        error,
        format!("{FIRST_CONTEXT}: {FAILURE_DETAIL}{FAILURE_SEPARATOR}{SECOND_CONTEXT}")
    );
}

#[test]
fn status_is_required_and_success_never_hides_recorded_failure() {
    let status = ExitStatus::from_raw(EMPTY_WAIT_STATUS);
    assert_eq!(Failures::default().finish(Some(status)).unwrap(), status);
    assert_eq!(
        Failures::default().finish(None).unwrap_err().to_string(),
        REAP_SKIPPED_DETAIL
    );
    let mut failures = Failures::default();
    assert!(
        failures
            .capture::<()>(Err(anyhow!(FAILURE_DETAIL)), FIRST_CONTEXT)
            .is_none()
    );
    assert!(
        failures
            .finish(None)
            .unwrap_err()
            .to_string()
            .contains(FAILURE_DETAIL)
    );
}

#[test]
fn expired_cleanup_pause_does_not_wait() {
    pause(Instant::now());
}

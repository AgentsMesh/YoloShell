use crate::fixture::{BEFORE_DEADLINE, begin, controller};
use uuid::Uuid;
use yolo_tui_terminal_control::{ATTEMPT_TIMEOUT, Failure, Presentation, RETRY_INTERVAL};

#[test]
fn transient_failures_preserve_intent_and_resume_when_connected() {
    let (mut control, now) = controller();
    let previous = begin(&mut control, now);
    assert!(control.failed(previous, Failure::Transient, now));
    assert_eq!(control.phase(now, false), Presentation::Recovering);
    assert!(!control.due(now, true, false));
    let retry = now + RETRY_INTERVAL;
    assert!(!control.due(retry, false, false));
    assert!(!control.due(retry, true, true));
    assert!(control.due(retry, true, false));
    let current = begin(&mut control, retry);
    assert!(!control.failed(previous, Failure::Denied, retry));
    assert!(control.succeeded(current, retry));
}

#[test]
fn pending_acquisition_has_a_finite_deadline_and_rejects_a_success_at_the_boundary() {
    let (mut control, now) = controller();
    let attempt = begin(&mut control, now);
    let deadline = now + ATTEMPT_TIMEOUT;
    assert_eq!(
        control.phase(deadline - BEFORE_DEADLINE, false),
        Presentation::Acquiring
    );
    assert!(!control.due(deadline - BEFORE_DEADLINE, true, false));
    assert_eq!(control.phase(deadline, false), Presentation::Recovering);
    assert!(!control.succeeded(attempt, deadline));
    assert_eq!(control.pending(), None);
    assert_eq!(control.failure(), Some(Failure::Transient));
    assert!(!control.due(deadline, true, false));
    assert!(control.due(deadline + RETRY_INTERVAL, true, false));
}

#[test]
fn missing_completion_is_expired_by_polling_and_allows_exactly_one_new_attempt() {
    let (mut control, now) = controller();
    let attempt = begin(&mut control, now);
    let deadline = now + ATTEMPT_TIMEOUT;
    assert!(!control.due(deadline, true, false));
    assert!(!control.failed(attempt, Failure::Denied, deadline));
    let retry = deadline + RETRY_INTERVAL;
    assert!(control.due(retry, true, false));
    let current = begin(&mut control, retry);
    assert_eq!(control.pending(), Some(current));
    assert!(!control.due(retry, true, false));
}

#[test]
fn beginning_after_deadline_cannot_bypass_the_backoff() {
    let (mut control, now) = controller();
    begin(&mut control, now);
    let deadline = now + ATTEMPT_TIMEOUT;
    assert!(
        control
            .begin(Uuid::new_v4(), Uuid::new_v4(), deadline)
            .is_none()
    );
    assert_eq!(control.failure(), Some(Failure::Transient));
    assert!(control.due(deadline + RETRY_INTERVAL, true, false));
}

#[test]
fn repeated_loss_does_not_postpone_recovery_forever() {
    let (mut control, now) = controller();
    control.adopt(now);
    control.lost(now);
    let retry = now + RETRY_INTERVAL;
    control.lost(retry - BEFORE_DEADLINE);
    control.disconnected(retry);
    assert_eq!(control.phase(retry, false), Presentation::Recovering);
    assert!(control.due(retry, true, false));
    let attempt = begin(&mut control, retry);
    assert!(control.succeeded(attempt, retry));
    assert_eq!(control.phase(retry, true), Presentation::Writing);
}

#[test]
fn validated_renewal_failure_keeps_its_reason_when_generic_retirement_follows() {
    for failure in [Failure::Contended, Failure::Denied, Failure::Transient] {
        let (mut control, now) = controller();
        control.adopt(now);
        control.record_failure(failure, now);
        let retry = now + RETRY_INTERVAL;
        control.lost(retry);
        control.disconnected(retry);
        assert_eq!(control.failure(), Some(failure));
        assert_eq!(control.due(retry, true, false), failure != Failure::Denied);
    }
}

#[test]
fn observer_failure_cannot_manufacture_write_intent() {
    let (mut control, now) = controller();
    control.record_failure(Failure::Denied, now);
    assert_eq!(control.failure(), None);
    assert_eq!(control.phase(now, false), Presentation::ReadOnly);
    assert!(!control.due(now + RETRY_INTERVAL, true, false));
}

use crate::fixture::{begin, controller, target};
use uuid::Uuid;
use yolo_tui_terminal_control::{Attempt, Failure, Intent, Presentation, RETRY_INTERVAL};

#[test]
fn altered_target_request_or_connection_cannot_complete_the_current_attempt() {
    let (mut control, now) = controller();
    let expected = begin(&mut control, now);
    for forged in [
        Attempt {
            target: target(),
            ..expected
        },
        Attempt {
            request_id: Uuid::new_v4(),
            ..expected
        },
        Attempt {
            connection: Uuid::new_v4(),
            ..expected
        },
    ] {
        assert!(!control.succeeded(forged, now));
        assert!(!control.failed(forged, Failure::Denied, now));
        assert_eq!(control.pending(), Some(expected));
        assert_eq!(control.failure(), None);
    }
    assert!(control.succeeded(expected, now));
}

#[test]
fn explicit_observe_is_a_tombstone_against_late_grants_and_failures() {
    let (mut control, now) = controller();
    let attempt = begin(&mut control, now);
    control.observe();
    assert_eq!(control.intent(), Intent::Observe);
    assert_eq!(control.pending(), None);
    assert!(!control.succeeded(attempt, now));
    assert!(!control.failed(attempt, Failure::Transient, now));
    control.disconnected(now);
    assert_eq!(control.phase(now, false), Presentation::ReadOnly);
    assert_eq!(control.failure(), None);
    let replacement = begin(&mut control, now);
    assert!(!control.succeeded(attempt, now));
    assert_eq!(control.pending(), Some(replacement));
}

#[test]
fn reconnection_cannot_restore_old_attempt_even_when_target_has_not_changed() {
    let (mut control, now) = controller();
    let attempt = begin(&mut control, now);
    control.disconnected(now);
    assert_eq!(control.intent(), Intent::Write);
    assert_eq!(control.pending(), None);
    assert!(!control.succeeded(attempt, now));
    let retry = now + RETRY_INTERVAL;
    let current = begin(&mut control, retry);
    assert_eq!(current.target, attempt.target);
    assert_ne!(current.connection, attempt.connection);
    assert!(!control.failed(attempt, Failure::Denied, retry));
    assert_eq!(control.pending(), Some(current));
    assert!(control.succeeded(current, retry));
}

#[test]
fn validated_external_grant_retires_pending_attempt_and_clears_recovery() {
    let (mut control, now) = controller();
    let previous = begin(&mut control, now);
    control.adopt(now);
    assert_eq!(control.intent(), Intent::Write);
    assert_eq!(control.failure(), None);
    assert_eq!(control.pending(), None);
    assert_eq!(control.phase(now, true), Presentation::Writing);
    assert!(!control.failed(previous, Failure::Denied, now));
    assert!(!control.succeeded(previous, now));
    control.observe();
    control.adopt(now);
    assert_eq!(control.phase(now, true), Presentation::Writing);
}

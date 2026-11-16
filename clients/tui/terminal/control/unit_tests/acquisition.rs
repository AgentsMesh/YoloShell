use crate::fixture::{begin, controller};
use uuid::Uuid;
use yolo_tui_terminal_control::{Failure, Intent, Presentation, RETRY_INTERVAL};

#[test]
fn observation_has_no_automatic_write_intent_or_acquisition() {
    let (mut control, now) = controller();
    assert_eq!(control.intent(), Intent::Observe);
    assert_eq!(control.phase(now, false), Presentation::ReadOnly);
    assert_eq!(control.phase(now, true), Presentation::ReadOnly);
    assert_eq!(control.failure(), None);
    assert_eq!(control.pending(), None);
    assert!(!control.due(now, true, false));
    assert!(control.begin(Uuid::new_v4(), Uuid::new_v4(), now).is_none());
}

#[test]
fn explicit_write_acquires_once_and_preserves_its_exact_pending_identity() {
    let (mut control, now) = controller();
    control.request_write(now);
    assert_eq!(control.intent(), Intent::Write);
    assert_eq!(control.phase(now, false), Presentation::Acquiring);
    assert!(!control.due(now, false, false));
    assert!(!control.due(now, true, true));
    assert!(control.due(now, true, false));
    let attempt = begin(&mut control, now);
    assert_eq!(attempt.target, control.target());
    assert_eq!(control.pending(), Some(attempt));
    control.request_write(now);
    assert_eq!(control.pending(), Some(attempt));
    assert!(!control.due(now, true, false));
    assert!(control.begin(Uuid::new_v4(), Uuid::new_v4(), now).is_none());
    assert!(control.succeeded(attempt, now));
    assert_eq!(control.phase(now, true), Presentation::Writing);
    assert_eq!(control.phase(now, false), Presentation::Recovering);
    assert_eq!(control.pending(), None);
    assert!(!control.succeeded(attempt, now));
}

#[test]
fn nil_identifiers_never_create_pending_authority() {
    let (mut control, now) = controller();
    control.request_write(now);
    for (id, connection) in [(Uuid::nil(), Uuid::new_v4()), (Uuid::new_v4(), Uuid::nil())] {
        assert!(control.begin(id, connection, now).is_none());
        assert_eq!(control.pending(), None);
        assert!(control.due(now, true, false));
    }
}

#[test]
fn contention_is_visible_and_can_only_retry_ordinary_acquisition_at_a_bounded_rate() {
    let (mut control, now) = controller();
    let attempt = begin(&mut control, now);
    assert!(control.failed(attempt, Failure::Contended, now));
    assert_eq!(control.phase(now, false), Presentation::Occupied);
    assert_eq!(control.failure(), Some(Failure::Contended));
    control.request_write(now);
    assert!(!control.due(now, true, false));
    let retry = now + RETRY_INTERVAL;
    assert!(control.due(retry, true, false));
    let next = begin(&mut control, retry);
    assert_eq!(control.phase(retry, false), Presentation::Occupied);
    assert!(!control.succeeded(attempt, retry));
    assert!(control.succeeded(next, retry));
    assert_eq!(control.failure(), None);
}

#[test]
fn denial_requires_fresh_user_intent_and_disconnection_does_not_bypass_it() {
    let (mut control, now) = controller();
    let attempt = begin(&mut control, now);
    assert!(control.failed(attempt, Failure::Denied, now));
    let retry = now + RETRY_INTERVAL;
    control.disconnected(retry);
    assert_eq!(control.failure(), Some(Failure::Denied));
    assert_eq!(control.phase(retry, false), Presentation::Denied);
    assert!(!control.due(retry, true, false));
    assert!(
        control
            .begin(Uuid::new_v4(), Uuid::new_v4(), retry)
            .is_none()
    );
    control.request_write(retry);
    assert!(control.due(retry, true, false));
    assert_eq!(control.failure(), None);
}

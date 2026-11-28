use crate::{ControlError, RequestScope};
use std::{
    error::Error,
    time::{Duration, Instant},
};

const VALID_DURATION: Duration = Duration::from_secs(60);

#[test]
fn dropping_scope_retires_every_captured_validity_without_changing_deadline() {
    let scope = RequestScope::default();
    let deadline = Instant::now() + VALID_DURATION;
    let original = scope.validity_until(deadline);
    let queued = original.clone();
    assert!(original.is_current() && queued.is_current());
    drop(scope);
    assert!(!original.is_current() && !queued.is_current());
    assert_eq!(original.deadline(), deadline);
    assert_eq!(queued.deadline(), deadline);
}

#[test]
fn explicit_retirement_is_idempotent_and_cannot_issue_a_fresh_validity() {
    let scope = RequestScope::default();
    let captured = scope.validity_until(Instant::now() + VALID_DURATION);
    scope.retire();
    scope.retire();
    assert!(!captured.is_current());
    assert!(
        !scope
            .validity_until(Instant::now() + VALID_DURATION)
            .is_current()
    );
    let replacement = RequestScope::default();
    assert!(
        replacement
            .validity_until(Instant::now() + VALID_DURATION)
            .is_current()
    );
}

#[test]
fn deadline_expiration_does_not_retire_the_scope_or_another_deadline() {
    let scope = RequestScope::default();
    let now = Instant::now();
    let expired = scope.validity_until(now);
    let pending = scope.validity_until(now + VALID_DURATION);
    assert!(!expired.is_current());
    assert!(pending.is_current());
    drop(expired);
    assert!(pending.is_current());
}

#[test]
fn ordered_input_rejection_cancels_later_deadlines_and_cannot_be_renewed() {
    let scope = RequestScope::ordered_inputs();
    let now = Instant::now();
    let expired = scope.validity_until(now);
    let pending = scope.validity_until(now + VALID_DURATION);
    assert!(!expired.is_current());
    assert!(pending.is_current());
    expired.reject();
    expired.reject();
    assert!(!pending.is_current());
    assert!(!scope.validity_until(now + VALID_DURATION).is_current());
    assert!(expired.same_scope(&pending));
}

#[test]
fn ordinary_scope_rejection_does_not_cancel_independent_members() {
    let scope = RequestScope::default();
    let pending = scope.validity_until(Instant::now() + VALID_DURATION);
    pending.reject();
    assert!(pending.is_current());
    assert!(scope.validity_until(pending.deadline()).is_current());
}

#[test]
fn scope_identity_distinguishes_attempts_even_with_equal_deadlines() {
    let first = RequestScope::default();
    let second = RequestScope::default();
    let deadline = Instant::now() + VALID_DURATION;
    let original = first.validity_until(deadline);
    let replacement = second.validity_until(deadline);
    assert!(original.same_scope(&original.clone()));
    assert!(!original.same_scope(&replacement));
    assert!(!replacement.same_scope(&original));
    let expired = first.validity_until(Instant::now());
    assert!(original.same_scope(&expired));
    assert!(!expired.is_current());
    drop(first);
    assert!(original.same_scope(&expired));
    assert!(!original.is_current());
    assert!(!original.same_scope(&replacement));
    assert!(replacement.is_current());
}

#[test]
fn invalidated_queue_admission_has_a_definitive_local_error_without_client_cause() {
    let error = ControlError::Invalidated;
    assert!(error.source().is_none());
    assert_eq!(error.to_string(), crate::constants::INVALIDATED_REQUEST);
}

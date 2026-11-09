use crate::RequestScope;
use std::time::{Duration, Instant};

const LIVE_LEASE: Duration = Duration::from_secs(10);
const RENEWAL: Duration = Duration::from_secs(20);

#[test]
fn confirmed_renewal_extends_queued_admission_without_changing_attempt_identity() {
    let now = Instant::now();
    let scope = RequestScope::leased_inputs(now + LIVE_LEASE);
    let queued = scope.validity_until(now + LIVE_LEASE);
    let dispatch_deadline = queued.admission_deadline();
    assert!(scope.renew_authority(now + RENEWAL));
    assert_eq!(queued.deadline(), now + LIVE_LEASE);
    assert_eq!(queued.admission_deadline(), now + RENEWAL);
    assert_eq!(dispatch_deadline, now + LIVE_LEASE);
    assert!(queued.is_current());
}

#[test]
fn retired_rejected_or_expired_lease_cannot_be_revived_by_a_renewal() {
    let now = Instant::now();
    for expired in [false, true] {
        let expires = if expired { now } else { now + LIVE_LEASE };
        let scope = RequestScope::leased_inputs(expires);
        let queued = scope.validity_until(expires);
        if !expired {
            queued.reject();
        }
        assert!(!scope.renew_authority(now + RENEWAL));
        assert!(!queued.is_current());
        assert!(!scope.validity_until(now + RENEWAL).is_current());
    }
}

#[test]
fn unrelated_scope_renewal_cannot_extend_another_writer_or_plain_operation() {
    let now = Instant::now();
    let first = RequestScope::leased_inputs(now + LIVE_LEASE);
    let second = RequestScope::leased_inputs(now + LIVE_LEASE);
    let old = first.validity_until(now + LIVE_LEASE);
    assert!(second.renew_authority(now + RENEWAL));
    assert_eq!(old.admission_deadline(), now + LIVE_LEASE);
    assert!(!old.same_scope(&second.validity_until(now + LIVE_LEASE)));
}

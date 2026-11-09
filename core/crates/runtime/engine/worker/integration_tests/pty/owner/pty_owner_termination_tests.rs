use super::*;
use crate::worker::pty_fault_reaper::FaultReapCompletion;
use crate::worker::pty_force_termination::ForceTransactionState;
use crate::worker::pty_ownership::{FaultSettlement, ReapEvidence};
use std::time::Duration;

#[path = "../../../test_support/integration/pty/pty_owner_termination_test_support.rs"]
mod support;
use support::{GracefulFixture, wait_for_marker};

const DEADLINE_EPSILON: Duration = Duration::from_nanos(1);
const REPEATED_REQUEST_COUNT: u32 = 8;
const FORCE_DEADLINE_EXPECTED_DETAIL: &str = "force capability deadline did not expire";
const FORCE_DEFERRED_STATUS_MISSING_DETAIL: &str =
    "deferred force termination did not preserve the root exit status";
const FORCE_DEFERRED_SETTLEMENT_TIMEOUT: Duration = Duration::from_secs(10);

#[test]
fn linux_custody_remains_graceful_for_worker_takeover_window() {
    let origin = Instant::now();
    let mut termination = OwnerTermination::new_at(origin);

    assert_eq!(
        termination.phase_at(origin, TerminationAuthority::LinuxContainment),
        TerminationPhase::Graceful
    );
    assert!(termination.capability_deadline.is_none());
}

#[test]
fn stubborn_session_escalates_only_at_the_graceful_deadline() {
    let origin = Instant::now();
    let mut termination = OwnerTermination::new_at(origin);
    let before = origin + GRACEFUL_TERMINATION_TIMEOUT - DEADLINE_EPSILON;

    assert_eq!(
        termination.phase_at(before, TerminationAuthority::LinuxContainment),
        TerminationPhase::Graceful
    );
    assert_eq!(
        termination.phase_at(
            origin + GRACEFUL_TERMINATION_TIMEOUT,
            TerminationAuthority::LinuxContainment,
        ),
        TerminationPhase::Force
    );
}

#[test]
fn repeated_ordinary_requests_cannot_postpone_escalation() {
    let origin = Instant::now();
    let mut termination = OwnerTermination::new_at(origin);
    for observation in 0..REPEATED_REQUEST_COUNT {
        let now = origin + PROCESS_SCOPE_REFRESH_INTERVAL.saturating_mul(observation);
        termination.request_at(false, now);
        assert_eq!(
            termination.phase_at(now, TerminationAuthority::LinuxContainment),
            TerminationPhase::Graceful
        );
    }
    assert_eq!(
        termination.phase_at(
            origin + GRACEFUL_TERMINATION_TIMEOUT,
            TerminationAuthority::LinuxContainment,
        ),
        TerminationPhase::Force
    );
}

#[test]
fn unavailable_authority_enters_force_without_a_graceful_signal() {
    let origin = Instant::now();
    let mut termination = OwnerTermination::new_at(origin);

    assert_eq!(
        termination.phase_at(origin, TerminationAuthority::Unavailable),
        TerminationPhase::Force
    );
}

#[test]
fn explicit_force_bypasses_the_graceful_deadline() {
    let origin = Instant::now();
    let mut termination = OwnerTermination::new_at(origin);
    termination.request_at(true, origin);
    assert_eq!(
        termination.phase_at(origin, TerminationAuthority::LinuxContainment),
        TerminationPhase::Force
    );
}

#[test]
fn force_capability_deadline_returns_a_typed_deferred_fault() {
    let mut fixture = GracefulFixture::spawn();
    let origin = Instant::now();
    let mut termination = OwnerTermination::new_at(origin);
    termination.request_at(true, origin);
    termination.capability_deadline = Some(CapabilitySettlementDeadline::immediate(origin));

    let error = match termination.poll_at(origin, &mut fixture.ownership, &mut fixture.output) {
        Err(error) => error,
        Ok(_) => panic!("{FORCE_DEADLINE_EXPECTED_DETAIL}"),
    };

    assert!(
        error
            .downcast_ref::<CapabilitySettlementDeferred>()
            .is_some()
    );
    assert!(matches!(
        fixture.ownership.force_mut_for_test().transaction_state(),
        ForceTransactionState::Started
    ));
    let (completion, receiver) = FaultReapCompletion::owner_channel();
    assert!(matches!(
        fixture.ownership.settle_after_fault(completion),
        FaultSettlement::Deferred(_)
    ));
    let outcome = receiver
        .receive_timeout(FORCE_DEFERRED_SETTLEMENT_TIMEOUT)
        .unwrap();
    let status = outcome.status.expect(FORCE_DEFERRED_STATUS_MISSING_DETAIL);
    assert_eq!(outcome.evidence, ReapEvidence::Waited);
    assert_eq!(status.signal(), Some(libc::SIGKILL));
    assert!(outcome.fault.is_none());
    assert!(!fixture.root.matches());
}

#[test]
fn ordinary_request_delivers_term_before_the_force_deadline() {
    let mut fixture = GracefulFixture::spawn();
    let origin = Instant::now();
    let mut termination = OwnerTermination::new_at(origin);

    assert!(matches!(
        termination
            .poll_at(origin, &mut fixture.ownership, &mut fixture.output)
            .unwrap(),
        ForceTerminationPoll::Pending
    ));
    assert!(termination.capability_deadline.is_none());
    wait_for_marker(&fixture.term_marker);

    assert_eq!(
        termination.phase_at(
            origin + PROCESS_SCOPE_REFRESH_INTERVAL,
            TerminationAuthority::LinuxContainment,
        ),
        TerminationPhase::Graceful
    );
    assert!(termination.capability_deadline.is_none());
    assert_eq!(
        termination.phase_at(
            origin + GRACEFUL_TERMINATION_TIMEOUT,
            TerminationAuthority::LinuxContainment,
        ),
        TerminationPhase::Force
    );
    fixture.settle();
}

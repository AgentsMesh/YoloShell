use super::*;
use crate::worker::pty_fault_reaper::bootstrap_failure::{self, BootstrapFailurePoll};
use crate::worker::pty_fault_reaper::live_authority;
use crate::worker::pty_force_termination::ForceTransactionState;

#[path = "../test_support/integration/explicit_bootstrap/explicit_bootstrap_initial_signal_fixture.rs"]
mod fixture;
use fixture::{ExplicitFailureFixture, FailureKind};

#[test]
fn explicit_bootstrap_failure_signals_a_live_exclusive_root_and_completes() {
    let mut fixture = ExplicitFailureFixture::new(FailureKind::Reported);
    assert!(fixture.root.matches());
    fixture.assert_pending_before_exit();

    assert!(request(&mut fixture.ownership));

    fixture.observe_root_exit();
    fixture.finish(libc::SIGHUP);
}

#[test]
fn cancelled_bootstrap_release_signals_a_live_exclusive_root_and_completes() {
    let mut fixture = ExplicitFailureFixture::new(FailureKind::CancelledBeforeRelease);
    assert!(fixture.root.matches());
    fixture.assert_pending_before_exit();

    assert!(request(&mut fixture.ownership));

    fixture.observe_root_exit();
    fixture.finish(libc::SIGHUP);
}

#[test]
fn proven_root_exit_keeps_explicit_bootstrap_retirement_without_another_signal() {
    let mut fixture = ExplicitFailureFixture::new(FailureKind::Reported);
    assert!(
        fixture
            .ownership
            .custody()
            .authority
            .as_ref()
            .unwrap()
            .request_force_termination()
            .unwrap()
    );
    fixture.observe_root_exit();

    assert!(!request(&mut fixture.ownership));

    fixture.finish(libc::SIGKILL);
}

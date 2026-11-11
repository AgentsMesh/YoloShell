use crate::worker::pty_fault_reaper::{
    FaultReapCompletion, FaultReaperReservation, HeldFaultReaper,
};
use crate::worker::pty_output_progress::{OutputQuiescenceFence, ReaderBackedPtyOutput};
use crate::worker::pty_owner_cleanup::{RootExitCleanup, force_descendants_for_test};
use crate::worker::pty_owner_termination::OwnerTermination;
use crate::worker::pty_ownership::{FaultSettlement, PtyOwnership, ReapEvidence};
use crate::worker::pty_process_custody::PtyProcessCustodyKind;
use crate::worker::termination_timing::{
    CapabilitySettlementDeadline, CapabilitySettlementDeferred,
};
const HANDOFF_COMPLETION_TIMEOUT: Duration = Duration::from_secs(10);
const FORCE_HANDOFF_DEFERRED_DETAIL: &str = "force transaction handoff was not deferred";
const FORCE_HANDOFF_FROZEN_DETAIL: &str = "force transaction did not retain frozen custody";
const FORCE_HANDOFF_FENCE_DETAIL: &str = "force transaction did not retain its output fence";
const FORCE_HANDOFF_KERNEL_EOF_DETAIL: &str =
    "force transaction did not retain kernel EOF validation authority";
const FORCE_DEADLINE_EXPECTED_DETAIL: &str = "force capability deadline did not expire";

include!("../../../test_support/integration/pty/pty_force_termination_handoff_test_support.rs");
include!("pty_force_termination_fence_handoff_fault_tests.rs");

#[test]
fn owner_capability_deadline_hands_off_pending_kernel_eof_force() {
    let (mut ownership, mut output, held, mut eof_output, version) = pending_kernel_eof_fixture();
    let mut termination = OwnerTermination::new();
    termination.request(true);
    termination.expire_capability_deadline_for_test();

    let error = match termination.poll(&mut ownership, &mut output) {
        Err(error) => error,
        Ok(_) => panic!("{FORCE_DEADLINE_EXPECTED_DETAIL}"),
    };

    assert!(
        error
            .downcast_ref::<CapabilitySettlementDeferred>()
            .is_some()
    );
    handoff_and_assert(ownership, held, &mut eof_output, version);
}

#[test]
fn root_exit_cleanup_deadline_hands_off_pending_kernel_eof_force() {
    let (mut ownership, mut output, held, mut eof_output, version) = pending_kernel_eof_fixture();
    let deadline = CapabilitySettlementDeadline::immediate(Instant::now());

    let cleanup = force_descendants_for_test(&mut ownership, &mut output, deadline).unwrap();

    assert!(matches!(cleanup, RootExitCleanup::Deferred(_)));
    handoff_and_assert(ownership, held, &mut eof_output, version);
}

#[test]
fn reaper_driver_panic_preserves_pending_kernel_eof_force() {
    let (mut ownership, _output, held, mut eof_output, version) = pending_kernel_eof_fixture();
    let (completion, receiver) = FaultReapCompletion::owner_channel();
    assert!(matches!(
        ownership.settle_after_fault(completion),
        FaultSettlement::Deferred(_)
    ));
    let mut handoff = held.wait_for_handoff();
    let observed = handoff.observe_completion_for_test();
    drop(receiver);
    handoff.resume_after_driver_panic_for_test(move |force, custody| {
        assert_force_transaction(force, custody, &mut eof_output, &version);
        prepare_for_cleanup(force, custody);
    });
    let outcome = observed
        .receive_timeout(HANDOFF_COMPLETION_TIMEOUT)
        .unwrap();
    assert_eq!(outcome.evidence, ReapEvidence::Waited);
    assert!(outcome.status.is_some());
    assert!(outcome.fault.is_some());
}

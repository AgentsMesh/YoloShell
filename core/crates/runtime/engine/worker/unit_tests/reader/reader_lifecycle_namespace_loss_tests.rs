use crate::namespace_lease::{NamespaceLease, NamespaceOwnershipLoss};
use crate::paths::directory::PrivateDirectory;
use crate::worker::pty_output_progress::{
    OutputProgressState, OutputQuiescenceState, ReaderBackedPtyOutput,
};
use crate::worker::reader_lifecycle::namespace_loss::NamespaceLossDrainAdmission;
use crate::worker::reader_lifecycle::{PendingOutput, ReaderLifecycle};
use std::sync::Arc;

const LOSS_MANAGED_COMPONENT: &str = "managed";
const LOSS_DISPLACED_COMPONENT: &str = "displaced";
const PENDING_TEST_OUTPUT: &[u8] = b"pending-output";

fn namespace_loss_proof() -> NamespaceOwnershipLoss {
    let root = tempfile::tempdir().unwrap();
    let managed = root.path().join(LOSS_MANAGED_COMPONENT);
    let displaced = root.path().join(LOSS_DISPLACED_COMPONENT);
    PrivateDirectory::ensure(&managed).unwrap();
    let lease = NamespaceLease::capture(&[(LOSS_MANAGED_COMPONENT, &managed)]).unwrap();
    std::fs::rename(managed, displaced).unwrap();
    lease.prove_ownership_loss().unwrap()
}

#[test]
fn namespace_loss_never_discards_staged_output() {
    let lifecycle = ReaderLifecycle::default();
    lifecycle.lock_progress().pending = vec![PendingOutput {
        data: PENDING_TEST_OUTPUT.to_vec(),
        attempt: None,
    }];

    assert_eq!(
        lifecycle.begin_namespace_loss_drain(namespace_loss_proof()),
        NamespaceLossDrainAdmission::Blocked
    );
    assert_eq!(
        lifecycle.output_progress_state(),
        OutputProgressState::Pending
    );
}

#[test]
fn namespace_loss_preserves_real_eof_without_false_output_completion() {
    let lifecycle = Arc::new(ReaderLifecycle::default());
    lifecycle.mark_eof_observed();
    assert_eq!(
        lifecycle.begin_namespace_loss_drain(namespace_loss_proof()),
        NamespaceLossDrainAdmission::Entered
    );
    assert_eq!(
        lifecycle.output_progress_state(),
        OutputProgressState::Abandoned
    );
    assert!(!lifecycle.is_cancelled());
    assert!(lifecycle.kernel_eof_identity().is_some());

    let observed = Arc::new(std::sync::atomic::AtomicUsize::default());
    let mut output =
        ReaderBackedPtyOutput::observed_with_lifecycle(Arc::clone(&observed), lifecycle).recovery();
    let mut fence = output.begin_quiescence_fence().unwrap();
    assert_eq!(
        output.poll_quiescence_fence(&mut fence).unwrap(),
        OutputQuiescenceState::Reached
    );
}

#[test]
fn namespace_loss_preserves_an_in_flight_kernel_read() {
    let lifecycle = ReaderLifecycle::default();
    lifecycle.lock_progress().output_read_in_flight = true;

    assert_eq!(
        lifecycle.begin_namespace_loss_drain(namespace_loss_proof()),
        NamespaceLossDrainAdmission::Blocked
    );
    assert_eq!(
        lifecycle.output_progress_state(),
        OutputProgressState::Pending
    );
    assert!(!lifecycle.is_cancelled());
}

#[test]
fn namespace_loss_before_eof_never_invents_eof_authority() {
    let lifecycle = ReaderLifecycle::default();

    assert_eq!(
        lifecycle.begin_namespace_loss_drain(namespace_loss_proof()),
        NamespaceLossDrainAdmission::Entered
    );
    assert!(lifecycle.kernel_eof_identity().is_none());
    assert_eq!(
        lifecycle.output_progress_state(),
        OutputProgressState::Abandoned
    );
}

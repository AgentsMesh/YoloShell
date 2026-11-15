use super::pty_owner_handle::{
    OwnerSettlement, PtyOwnerCommand, PtyOwnerCommandFailure, PtyOwnerHandle,
};
use std::sync::mpsc;

#[path = "pty_owner_handle_settlement_tests.rs"]
mod settlement_tests;

const AVAILABLE_CAPABILITY_COUNT: usize = 1;
const NO_CAPABILITIES: usize = 0;
const OWNER_FAILURE_DETAIL: &str = "injected owner failure";

fn settlement() -> OwnerSettlement {
    crate::worker::pty_owner_handle::new_owner_settlement_for_test()
}

#[test]
fn destructive_capability_reports_and_preflights_exactly() {
    let (sender, _receiver) = mpsc::channel();
    let available = PtyOwnerHandle::for_test(sender, settlement());
    assert_eq!(
        available.runtime_capabilities(),
        [crate::protocol::capability::SAFE_PROCESS_TREE_TERMINATION]
    );
    available.preflight_destructive().unwrap();

    let (sender, _receiver) = mpsc::channel();
    let unavailable = PtyOwnerHandle::unavailable_for_test(sender, settlement());
    assert_eq!(unavailable.runtime_capabilities().len(), NO_CAPABILITIES);
    assert!(
        unavailable
            .preflight_destructive()
            .unwrap_err()
            .downcast_ref::<crate::process::DestructiveSessionUnavailable>()
            .is_some()
    );
    assert_eq!(
        available.runtime_capabilities().len(),
        AVAILABLE_CAPABILITY_COUNT
    );
}

#[tokio::test]
async fn command_submission_preserves_force_and_completion_result() {
    for (force, completion) in [
        (false, Ok(())),
        (
            true,
            Err(PtyOwnerCommandFailure::Owner(
                OWNER_FAILURE_DETAIL.to_owned(),
            )),
        ),
    ] {
        let (sender, receiver) = mpsc::channel();
        let handle = PtyOwnerHandle::for_test(sender, settlement());
        let submission = handle.submit_termination(force).unwrap();
        let PtyOwnerCommand::Terminate {
            force: observed,
            response,
        } = receiver.recv().unwrap();
        assert_eq!(observed, force);
        response.send(completion).unwrap();
        let result = submission.wait().await;
        if force {
            assert_eq!(result.unwrap_err().to_string(), OWNER_FAILURE_DETAIL);
        } else {
            result.unwrap();
        }
    }
}

#[tokio::test]
async fn cleanup_deferred_command_completion_remains_typed() {
    let (sender, receiver) = mpsc::channel();
    let handle = PtyOwnerHandle::for_test(sender, settlement());
    let submission = handle.submit_termination(true).unwrap();
    let PtyOwnerCommand::Terminate { response, .. } = receiver.recv().unwrap();
    response
        .send(Err(PtyOwnerCommandFailure::CleanupDeferred(
            super::termination_timing::CapabilitySettlementDeferred,
        )))
        .unwrap();

    let error = submission.wait().await.unwrap_err();

    assert!(
        error
            .downcast_ref::<super::termination_timing::CapabilitySettlementDeferred>()
            .is_some()
    );
}

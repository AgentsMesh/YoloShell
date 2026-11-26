use crate::host::tests::*;
const LIFECYCLE_WAVE6_NAME: &str = "lifecycle-wave6";

#[derive(Clone, Copy, Debug)]
enum LifecycleWave6Operation {
    Archive,
    Discard,
}

#[derive(Clone, Copy, Debug)]
enum LifecycleWave6State {
    Ready,
    Terminated,
    Archived,
    Finalized,
    Survivor,
}

#[tokio::test]
async fn lifecycle_cleanup_covers_every_durable_state_for_both_operations() {
    for operation in [
        LifecycleWave6Operation::Archive,
        LifecycleWave6Operation::Discard,
    ] {
        for state in [
            LifecycleWave6State::Ready,
            LifecycleWave6State::Terminated,
            LifecycleWave6State::Archived,
            LifecycleWave6State::Finalized,
            LifecycleWave6State::Survivor,
        ] {
            assert_lifecycle_wave6(operation, state).await;
        }
    }
}

#[tokio::test]
async fn cleanup_termination_honors_terminal_receipt_states() {
    let archived = completed_cleanup_termination(CreationCompletion::Archived).await;
    assert_eq!(archived.unwrap(), CleanupTerminationOutcome::Terminated);

    let finalized = completed_cleanup_termination(CreationCompletion::Finalized).await;
    assert_eq!(
        finalized.unwrap_err().code,
        crate::protocol::error_code::CONFLICT
    );
}

async fn completed_cleanup_termination(
    completion: CreationCompletion,
) -> std::result::Result<CleanupTerminationOutcome, ErrorBody> {
    let (_directory, host, _request, binding, _manifest, _runtime, _ledger_path) =
        lifecycle_product_fixture(LIFECYCLE_WAVE6_NAME, CreationPhase::Ready).await;
    host.creation_receipts
        .publish(&CreationReceipt::from_binding(&binding, completion))
        .unwrap();
    host.cleanup_terminate_session(CreationCleanupTerminateRequest {
        terminal_id: binding.result.terminal_id,
        creation_capability: binding.result.creation_capability,
        force: false,
    })
    .await
}

async fn assert_lifecycle_wave6(operation: LifecycleWave6Operation, state: LifecycleWave6State) {
    let phase = if matches!(state, LifecycleWave6State::Terminated) {
        CreationPhase::Terminated
    } else {
        CreationPhase::Ready
    };
    let (_directory, host, _request, binding, _manifest, _runtime, _ledger_path) =
        lifecycle_product_fixture(LIFECYCLE_WAVE6_NAME, phase).await;
    if let Some(completion) = lifecycle_wave6_completion(state) {
        host.creation_receipts
            .publish(&CreationReceipt::from_binding(&binding, completion))
            .unwrap();
    }

    let result = match operation {
        LifecycleWave6Operation::Archive => {
            host.cleanup_archive_session(CreationCleanupArchiveRequest {
                terminal_id: binding.result.terminal_id,
                creation_capability: binding.result.creation_capability,
            })
            .await
        }
        LifecycleWave6Operation::Discard => {
            host.cleanup_discard_session(CreationCleanupDiscardRequest {
                terminal_id: binding.result.terminal_id,
                creation_capability: binding.result.creation_capability,
                force: false,
            })
            .await
        }
    };
    let expected_success = match (operation, state) {
        (LifecycleWave6Operation::Archive, LifecycleWave6State::Ready)
        | (LifecycleWave6Operation::Archive, LifecycleWave6State::Finalized)
        | (LifecycleWave6Operation::Archive, LifecycleWave6State::Survivor)
        | (LifecycleWave6Operation::Discard, LifecycleWave6State::Finalized) => false,
        _ => true,
    };
    assert_eq!(result.is_ok(), expected_success, "{operation:?} {state:?}");
}

fn lifecycle_wave6_completion(state: LifecycleWave6State) -> Option<CreationCompletion> {
    match state {
        LifecycleWave6State::Ready | LifecycleWave6State::Terminated => None,
        LifecycleWave6State::Archived => Some(CreationCompletion::Archived),
        LifecycleWave6State::Finalized => Some(CreationCompletion::Finalized),
        LifecycleWave6State::Survivor => Some(CreationCompletion::Survivor),
    }
}

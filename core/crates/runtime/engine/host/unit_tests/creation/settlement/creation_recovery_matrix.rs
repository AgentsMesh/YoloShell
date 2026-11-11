use crate::host::tests::*;
use super::*;

#[derive(Clone, Copy, Debug)]
struct ReceiptRecoveryCase {
    completion: CreationCompletion,
    phase: CreationPhase,
    publish_token_index: bool,
}

#[test]
fn recovery_repairs_partial_receipts_and_removes_completed_bindings() {
    let cases = [
        ReceiptRecoveryCase {
            completion: CreationCompletion::Finalized,
            phase: CreationPhase::Ready,
            publish_token_index: true,
        },
        ReceiptRecoveryCase {
            completion: CreationCompletion::Archived,
            phase: CreationPhase::Terminated,
            publish_token_index: false,
        },
    ];
    for case in cases {
        let (_directory, store, mut binding, path) = receipt_test_setup();
        binding.phase = case.phase;
        let mut ledger = transition_ledger(&path, &binding);
        let receipt = CreationReceipt::from_binding(&binding, case.completion);
        let (index, name) = if case.publish_token_index {
            (&store.token_index, binding.creation_token_digest.clone())
        } else {
            (
                &store.session_index,
                binding.result.terminal_id.hyphenated().to_string(),
            )
        };
        publish_receipt_at(&store.staging, index, &name, &receipt).unwrap();

        ledger.recover_completed(&path, &store).unwrap();
        assert_transition_ledger(&path, &[]);
        assert_eq!(store.lookup_binding(&binding).unwrap(), Some(receipt));
    }
}

#[test]
fn recovery_without_receipt_or_binding_is_a_durable_noop() {
    let (_directory, store, binding, path) = receipt_test_setup();
    let mut ledger = transition_ledger(&path, &binding);
    ledger.recover_completed(&path, &store).unwrap();
    ledger
        .recover_token_completion(&path, &store, Uuid::new_v4())
        .unwrap();
    assert!(
        ledger
            .recover_session_completion(&path, &store, Uuid::new_v4())
            .unwrap()
            .is_none()
    );
    assert_transition_ledger(&path, &[binding]);
}

#[test]
fn finalized_receipt_conflicting_with_terminated_phase_fails_closed() {
    let (_directory, store, mut binding, path) = receipt_test_setup();
    binding.phase = CreationPhase::Terminated;
    let mut ledger = transition_ledger(&path, &binding);
    let receipt = CreationReceipt::from_binding(&binding, CreationCompletion::Finalized);
    store.publish(&receipt).unwrap();

    assert!(ledger.recover_completed(&path, &store).is_err());
    let error = ledger
        .recover_session_completion(&path, &store, binding.result.terminal_id)
        .unwrap_err();
    assert_eq!(error.code, crate::protocol::error_code::INTERNAL);
    assert_transition_ledger(&path, &[binding]);
}

#[test]
fn poisoned_ledger_rejects_all_product_state_queries() {
    let (_directory, store, binding, path) = receipt_test_setup();
    let mut ledger = transition_ledger(&path, &binding);
    ledger.poisoned = true;
    let request = FinalizeCreationRequest {
        creation_token: Uuid::new_v4(),
        terminal_id: binding.result.terminal_id,
        creation_capability: binding.result.creation_capability,
    };
    let errors = [
        ledger.prepare_launch(&path, binding.result).unwrap_err(),
        ledger
            .set_active_phase(&path, binding.result, CreationPhase::Ready)
            .unwrap_err(),
        ledger
            .active_creation(binding.result.terminal_id)
            .unwrap_err(),
        ledger.pending_finalize(&request, &store).unwrap_err(),
        ledger
            .cleanup_state(
                &path,
                binding.result.terminal_id,
                binding.result.creation_capability,
                &store,
            )
            .unwrap_err(),
        ledger
            .complete_product_archive(&path, binding.result, &store)
            .unwrap_err(),
    ];
    for error in errors {
        assert_eq!(error.code, crate::protocol::error_code::INTERNAL);
    }
    assert_transition_ledger(&path, &[binding]);
}

#[test]
fn product_archive_is_idempotent_when_no_active_binding_remains() {
    let (_directory, store, binding, path) = receipt_test_setup();
    let mut ledger = CreationLedger::default();
    ledger
        .complete_product_archive(&path, binding.result, &store)
        .unwrap();
    assert!(ledger.bindings.is_empty());
    assert!(store.lookup_binding(&binding).unwrap().is_none());
}

use crate::host::tests::*;
use super::*;

pub(super) fn finalize_request_for(token: Uuid, binding: &CreationBinding) -> FinalizeCreationRequest {
    FinalizeCreationRequest {
        creation_token: token,
        terminal_id: binding.result.terminal_id,
        creation_capability: binding.result.creation_capability,
    }
}

#[test]
fn finalize_rejects_each_non_ready_phase_without_durable_mutation() {
    for phase in [
        CreationPhase::Reserved,
        CreationPhase::Launching,
        CreationPhase::SpawnFailed,
        CreationPhase::Terminated,
    ] {
        let (_directory, store, mut binding, path) = receipt_test_setup();
        let token = Uuid::new_v4();
        binding.creation_token_digest = creation_token_digest(token);
        binding.phase = phase;
        if phase == CreationPhase::Reserved {
            binding.launch_generation = RESERVED_CREATION_LAUNCH_GENERATION;
            binding.launch = None;
        }
        let mut ledger = transition_ledger(&path, &binding);
        let request = finalize_request_for(token, &binding);
        for error in [
            ledger.pending_finalize(&request, &store).unwrap_err(),
            ledger.finalize(&path, request, &store).unwrap_err(),
        ] {
            assert_eq!(error.code, crate::protocol::error_code::CONFLICT);
        }
        assert_transition_ledger(&path, &[binding.clone()]);
        assert_receipt_indexes_empty(&store);
    }
}

#[test]
fn finalize_identity_matrix_fails_closed_and_preserves_ready_binding() {
    let mutations: [fn(&mut FinalizeCreationRequest); 3] = [
        |request| request.creation_token = Uuid::new_v4(),
        |request| request.terminal_id = Uuid::new_v4(),
        |request| request.creation_capability = Uuid::new_v4(),
    ];
    for mutate in mutations {
        let (_directory, store, mut binding, path) = receipt_test_setup();
        let token = Uuid::new_v4();
        binding.creation_token_digest = creation_token_digest(token);
        let mut ledger = transition_ledger(&path, &binding);
        let mut request = finalize_request_for(token, &binding);
        mutate(&mut request);
        for error in [
            ledger.pending_finalize(&request, &store).unwrap_err(),
            ledger.finalize(&path, request, &store).unwrap_err(),
        ] {
            assert_eq!(error.code, crate::protocol::error_code::UNAUTHORIZED);
        }
        assert_transition_ledger(&path, &[binding]);
        assert_receipt_indexes_empty(&store);
    }
}

#[test]
fn finalize_success_and_receipt_replay_converge_on_empty_ledger() {
    let (_directory, store, mut binding, path) = receipt_test_setup();
    let token = Uuid::new_v4();
    binding.creation_token_digest = creation_token_digest(token);
    let mut ledger = transition_ledger(&path, &binding);
    let request = finalize_request_for(token, &binding);
    assert_eq!(
        ledger.pending_finalize(&request, &store).unwrap(),
        Some(binding.clone())
    );
    ledger.finalize(&path, request, &store).unwrap();
    assert_transition_ledger(&path, &[]);
    assert_eq!(
        store.lookup_binding(&binding).unwrap().unwrap().completion,
        CreationCompletion::Finalized
    );

    ledger.finalize(&path, request, &store).unwrap();
    write_active_binding(&path, &binding);
    let mut replay = CreationLedger::open(&path).unwrap();
    assert!(replay.pending_finalize(&request, &store).unwrap().is_none());
    replay.finalize(&path, request, &store).unwrap();
    assert_transition_ledger(&path, &[]);
}

#[test]
fn archived_receipt_never_authorizes_finalize_replay() {
    let (_directory, store, mut binding, path) = receipt_test_setup();
    let token = Uuid::new_v4();
    binding.creation_token_digest = creation_token_digest(token);
    write_active_binding(&path, &binding);
    store
        .publish(&CreationReceipt::from_binding(
            &binding,
            CreationCompletion::Archived,
        ))
        .unwrap();
    let request = finalize_request_for(token, &binding);
    let mut ledger = CreationLedger::open(&path).unwrap();
    for error in [
        ledger.pending_finalize(&request, &store).unwrap_err(),
        ledger.finalize(&path, request, &store).unwrap_err(),
    ] {
        assert_eq!(error.code, crate::protocol::error_code::UNAUTHORIZED);
    }
    assert_transition_ledger(&path, &[binding]);
}

#[test]
fn completed_binding_removal_is_idempotent_for_unknown_product() {
    let (_directory, _store, binding, path) = receipt_test_setup();
    let mut ledger = transition_ledger(&path, &binding);
    let unknown = CreateTerminalResult {
        terminal_id: Uuid::new_v4(),
        creation_capability: Uuid::new_v4(),
    };
    ledger.remove_completed_binding(&path, unknown).unwrap();
    assert_transition_ledger(&path, &[binding]);
}

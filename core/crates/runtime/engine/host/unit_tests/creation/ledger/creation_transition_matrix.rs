use crate::host::tests::*;
use super::*;

pub(super) fn transition_ledger(path: &Path, binding: &CreationBinding) -> CreationLedger {
    write_active_binding(path, binding);
    CreationLedger::open(path).unwrap()
}

#[test]
fn launch_and_phase_transitions_are_durable_and_identity_bound() {
    let (directory, store, mut binding, path) = receipt_test_setup();
    binding.phase = CreationPhase::Reserved;
    binding.launch_generation = RESERVED_CREATION_LAUNCH_GENERATION;
    binding.launch = None;
    let mut ledger = transition_ledger(&path, &binding);
    let first = ledger.prepare_launch(&path, binding.result).unwrap();
    let replay = ledger.prepare_launch(&path, binding.result).unwrap();
    assert_eq!(replay, first);
    let stored = &CreationLedger::open(&path).unwrap().bindings[0];
    assert_eq!(stored.phase, CreationPhase::Launching);
    assert_eq!(stored.launch.as_ref(), Some(&first));

    ledger
        .set_active_phase(&path, binding.result, CreationPhase::Ready)
        .unwrap();
    assert_eq!(
        CreationLedger::open(&path).unwrap().bindings[0].phase,
        CreationPhase::Ready
    );
    let unknown = CreateTerminalResult {
        terminal_id: Uuid::new_v4(),
        creation_capability: Uuid::new_v4(),
    };
    for error in [
        ledger.prepare_launch(&path, unknown).unwrap_err(),
        ledger
            .set_active_phase(&path, unknown, CreationPhase::Terminated)
            .unwrap_err(),
    ] {
        assert_eq!(error.code, crate::protocol::error_code::UNAUTHORIZED);
    }
    assert_eq!(
        ledger.active_creation(binding.result.terminal_id).unwrap(),
        Some((binding.result, CreationPhase::Ready))
    );
    assert!(ledger.active_creation(Uuid::new_v4()).unwrap().is_none());
    drop((directory, store));
}

#[test]
fn cleanup_state_classifies_every_active_phase_without_mutation() {
    let phases = [
        (CreationPhase::Ready, Some(CleanupCreationState::Ready)),
        (
            CreationPhase::Terminated,
            Some(CleanupCreationState::Terminated),
        ),
        (CreationPhase::Reserved, None),
        (CreationPhase::Launching, None),
        (CreationPhase::SpawnFailed, None),
    ];
    for (phase, expected) in phases {
        let (_directory, store, mut binding, path) = receipt_test_setup();
        binding.phase = phase;
        if phase == CreationPhase::Reserved {
            binding.launch_generation = RESERVED_CREATION_LAUNCH_GENERATION;
            binding.launch = None;
        }
        let mut ledger = transition_ledger(&path, &binding);
        let result = ledger.cleanup_state(
            &path,
            binding.result.terminal_id,
            binding.result.creation_capability,
            &store,
        );
        match expected {
            Some(state) => assert_eq!(result.unwrap(), state),
            None => assert_eq!(
                result.unwrap_err().code,
                crate::protocol::error_code::CONFLICT
            ),
        }
        assert_transition_ledger(&path, std::slice::from_ref(&binding));
        let unauthorized = ledger
            .cleanup_state(&path, binding.result.terminal_id, Uuid::new_v4(), &store)
            .unwrap_err();
        assert_eq!(unauthorized.code, crate::protocol::error_code::UNAUTHORIZED);
    }
}

#[test]
fn cleanup_state_recovers_receipts_and_rejects_conflicting_completion() {
    for completion in [
        CreationCompletion::Archived,
        CreationCompletion::Finalized,
        CreationCompletion::Survivor,
    ] {
        let (_directory, store, binding, path) = receipt_test_setup();
        let mut ledger = transition_ledger(&path, &binding);
        let receipt = CreationReceipt::from_binding(&binding, completion);
        store.publish(&receipt).unwrap();
        let expected = match completion {
            CreationCompletion::Archived => CleanupCreationState::Archived,
            CreationCompletion::Finalized => CleanupCreationState::Finalized,
            CreationCompletion::Survivor => CleanupCreationState::Survivor,
        };
        assert_eq!(
            ledger
                .cleanup_state(
                    &path,
                    binding.result.terminal_id,
                    binding.result.creation_capability,
                    &store,
                )
                .unwrap(),
            expected
        );
        assert_transition_ledger(&path, &[]);
        let unauthorized = ledger
            .cleanup_state(&path, binding.result.terminal_id, Uuid::new_v4(), &store)
            .unwrap_err();
        assert_eq!(unauthorized.code, crate::protocol::error_code::UNAUTHORIZED);
        assert_eq!(
            ledger
                .cleanup_state(
                    &path,
                    binding.result.terminal_id,
                    binding.result.creation_capability,
                    &store,
                )
                .unwrap(),
            expected
        );
        assert_transition_ledger(&path, &[]);
    }

    let (_directory, store, mut binding, path) = receipt_test_setup();
    binding.phase = CreationPhase::Terminated;
    let mut ledger = transition_ledger(&path, &binding);
    store
        .publish(&CreationReceipt::from_binding(
            &binding,
            CreationCompletion::Finalized,
        ))
        .unwrap();
    let error = ledger
        .cleanup_state(
            &path,
            binding.result.terminal_id,
            binding.result.creation_capability,
            &store,
        )
        .unwrap_err();
    assert_eq!(error.code, crate::protocol::error_code::INTERNAL);
    assert_transition_ledger(&path, &[binding]);
}

#[test]
fn archive_completion_requires_terminated_binding_and_persists_receipt() {
    for phase in [
        CreationPhase::Reserved,
        CreationPhase::Launching,
        CreationPhase::SpawnFailed,
        CreationPhase::Ready,
    ] {
        let (_directory, store, mut binding, path) = receipt_test_setup();
        binding.phase = phase;
        if phase == CreationPhase::Reserved {
            binding.launch_generation = RESERVED_CREATION_LAUNCH_GENERATION;
            binding.launch = None;
        }
        let mut ledger = transition_ledger(&path, &binding);
        let error = ledger
            .complete_archive(&path, binding.result, &store)
            .unwrap_err();
        assert_eq!(error.code, crate::protocol::error_code::CONFLICT);
        assert_transition_ledger(&path, &[binding]);
        assert_receipt_indexes_empty(&store);
    }

    let (_directory, store, mut binding, path) = receipt_test_setup();
    binding.phase = CreationPhase::Terminated;
    let mut ledger = transition_ledger(&path, &binding);
    ledger
        .complete_archive(&path, binding.result, &store)
        .unwrap();
    assert_transition_ledger(&path, &[]);
    let receipt = store.lookup_binding(&binding).unwrap().unwrap();
    assert_eq!(receipt.completion, CreationCompletion::Archived);
}

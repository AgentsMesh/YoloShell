use crate::host::tests::*;
use super::*;

#[test]
fn prepare_launch_enforces_every_source_phase() {
    let cases = [
        (CreationPhase::Reserved, true),
        (CreationPhase::Launching, true),
        (CreationPhase::SpawnFailed, true),
        (CreationPhase::Ready, false),
        (CreationPhase::Terminated, false),
    ];
    for (phase, allowed) in cases {
        let (_directory, _store, mut binding, path) = receipt_test_setup();
        binding.phase = phase;
        if phase == CreationPhase::Reserved {
            binding.launch_generation = RESERVED_CREATION_LAUNCH_GENERATION;
            binding.launch = None;
        }
        let previous = binding.clone();
        let mut ledger = transition_ledger(&path, &binding);
        let outcome = ledger.prepare_launch(&path, binding.result);
        if allowed {
            let launch = outcome.unwrap();
            match phase {
                CreationPhase::Reserved => assert!(previous.launch.is_none()),
                CreationPhase::Launching => {
                    assert_eq!(Some(&launch), previous.launch.as_ref());
                }
                CreationPhase::SpawnFailed => {
                    let previous_launch = previous.launch.as_ref().unwrap();
                    assert_ne!(launch.runtime_epoch, previous_launch.runtime_epoch);
                    assert_ne!(launch.worker_id, previous_launch.worker_id);
                    assert_ne!(launch.auth_token, previous_launch.auth_token);
                }
                CreationPhase::Ready | CreationPhase::Terminated => unreachable!(),
            }
            let durable = CreationLedger::open(&path).unwrap();
            let stored = &durable.bindings[0];
            assert_eq!(stored.phase, CreationPhase::Launching);
            assert_eq!(stored.launch.as_ref(), Some(&launch));
            let expected_generation = if phase == CreationPhase::SpawnFailed {
                previous.launch_generation + INITIAL_CREATION_LAUNCH_GENERATION
            } else if phase == CreationPhase::Reserved {
                INITIAL_CREATION_LAUNCH_GENERATION
            } else {
                previous.launch_generation
            };
            assert_eq!(stored.launch_generation, expected_generation);
        } else {
            assert_eq!(
                outcome.unwrap_err().code,
                crate::protocol::error_code::CONFLICT
            );
            assert_transition_ledger(&path, &[previous]);
        }
    }
}

#[test]
fn active_phase_transition_matrix_rejects_every_regression() {
    let cases = [
        (CreationPhase::Reserved, CreationPhase::Reserved, true),
        (CreationPhase::Reserved, CreationPhase::Launching, false),
        (CreationPhase::Reserved, CreationPhase::SpawnFailed, false),
        (CreationPhase::Reserved, CreationPhase::Ready, false),
        (CreationPhase::Reserved, CreationPhase::Terminated, false),
        (CreationPhase::Launching, CreationPhase::Reserved, false),
        (CreationPhase::Launching, CreationPhase::Launching, true),
        (CreationPhase::Launching, CreationPhase::SpawnFailed, true),
        (CreationPhase::Launching, CreationPhase::Ready, true),
        (CreationPhase::Launching, CreationPhase::Terminated, true),
        (CreationPhase::SpawnFailed, CreationPhase::Reserved, false),
        (CreationPhase::SpawnFailed, CreationPhase::Launching, false),
        (CreationPhase::SpawnFailed, CreationPhase::SpawnFailed, true),
        (CreationPhase::SpawnFailed, CreationPhase::Ready, true),
        (CreationPhase::SpawnFailed, CreationPhase::Terminated, true),
        (CreationPhase::Ready, CreationPhase::Reserved, false),
        (CreationPhase::Ready, CreationPhase::Launching, false),
        (CreationPhase::Ready, CreationPhase::SpawnFailed, false),
        (CreationPhase::Ready, CreationPhase::Ready, true),
        (CreationPhase::Ready, CreationPhase::Terminated, true),
        (CreationPhase::Terminated, CreationPhase::Reserved, false),
        (CreationPhase::Terminated, CreationPhase::Launching, false),
        (CreationPhase::Terminated, CreationPhase::SpawnFailed, false),
        (CreationPhase::Terminated, CreationPhase::Ready, false),
        (CreationPhase::Terminated, CreationPhase::Terminated, true),
    ];
    for (source, target, allowed) in cases {
        let (_directory, _store, mut binding, path) = receipt_test_setup();
        binding.phase = source;
        if source == CreationPhase::Reserved {
            binding.launch_generation = RESERVED_CREATION_LAUNCH_GENERATION;
            binding.launch = None;
        }
        let previous = binding.clone();
        let mut ledger = transition_ledger(&path, &binding);
        let outcome = ledger.set_active_phase(&path, binding.result, target);
        if allowed {
            outcome.unwrap();
            let mut expected = previous;
            expected.phase = target;
            assert_transition_ledger(&path, &[expected]);
        } else {
            assert_eq!(
                outcome.unwrap_err().code,
                crate::protocol::error_code::CONFLICT
            );
            assert_transition_ledger(&path, &[previous]);
        }
    }
}

#[test]
fn durable_ledger_rejects_phase_launch_and_generation_inconsistency() {
    let cases = [
        (
            CreationPhase::Reserved,
            Some(CreationLaunchIdentity::new()),
            RESERVED_CREATION_LAUNCH_GENERATION,
        ),
        (
            CreationPhase::Reserved,
            None,
            INITIAL_CREATION_LAUNCH_GENERATION,
        ),
        (
            CreationPhase::Launching,
            None,
            INITIAL_CREATION_LAUNCH_GENERATION,
        ),
        (
            CreationPhase::SpawnFailed,
            None,
            INITIAL_CREATION_LAUNCH_GENERATION,
        ),
        (
            CreationPhase::Ready,
            None,
            INITIAL_CREATION_LAUNCH_GENERATION,
        ),
        (
            CreationPhase::Terminated,
            None,
            INITIAL_CREATION_LAUNCH_GENERATION,
        ),
        (
            CreationPhase::Launching,
            Some(CreationLaunchIdentity::new()),
            RESERVED_CREATION_LAUNCH_GENERATION,
        ),
    ];
    for (phase, launch, launch_generation) in cases {
        let (_directory, _store, mut binding, path) = receipt_test_setup();
        binding.phase = phase;
        binding.launch = launch;
        binding.launch_generation = launch_generation;
        write_active_binding(&path, &binding);
        assert!(CreationLedger::open(&path).is_err());
    }
}

use super::*;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

const NO_SPAWN_CALLS: usize = 0;
const SPAWN_CALL_INCREMENT: usize = 1;
const UNEXPECTED_SPAWN_DETAIL: &str =
    "spawn callback ran before emergency escrow reservation succeeded";

#[test]
fn emergency_registration_failure_prevents_child_spawn() {
    let spawn_calls = Arc::new(AtomicUsize::new(usize::default()));
    let callback_calls = Arc::clone(&spawn_calls);
    let mut cleanup_scope = StartupCleanupScope::new().unwrap();
    let spawn_guard = process::waitable_child_spawn_guard().unwrap();
    let boundary = spawn_guard.boundary();
    pty_fault_reaper::FaultReaperReservation::fail_next_emergency_reservation_for_test();

    let result = spawn_with_reserved_reaper(
        &mut cleanup_scope,
        move || {
            callback_calls.fetch_add(SPAWN_CALL_INCREMENT, Ordering::AcqRel);
            anyhow::bail!(UNEXPECTED_SPAWN_DETAIL)
        },
        &boundary,
        ExclusivePtyOutput::observed(Arc::new(AtomicUsize::new(usize::default()))),
        UnreleasedBootstrapGate,
    );

    assert!(result.is_err());
    assert_eq!(spawn_calls.load(Ordering::Acquire), NO_SPAWN_CALLS);
}

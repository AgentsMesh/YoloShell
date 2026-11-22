use super::*;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

const NO_SPAWN_CALLS: usize = 0;
const SINGLE_SPAWN_CALL: usize = 1;
const SPAWN_CALL_INCREMENT: usize = 1;
const UNEXPECTED_SPAWN_DETAIL: &str = "spawn callback ran without atomic signal capability";
const INJECTED_SPAWN_FAILURE: &str = "injected spawn callback failure";

fn output_progress() -> ExclusivePtyOutput {
    ExclusivePtyOutput::observed(Arc::new(AtomicUsize::default()))
}

#[tokio::test]
async fn unavailable_atomic_signal_capability_prevents_child_spawn() {
    let spawn_calls = Arc::new(AtomicUsize::default());
    let callback_calls = Arc::clone(&spawn_calls);
    let mut cleanup_scope = StartupCleanupScope::new().unwrap();
    let spawn_guard = process::waitable_child_spawn_guard().unwrap();
    let boundary = spawn_guard.boundary();

    let result = process::with_test_signal_capability(false, || {
        spawn_with_reserved_reaper(
            &mut cleanup_scope,
            move || {
                callback_calls.fetch_add(SPAWN_CALL_INCREMENT, Ordering::AcqRel);
                anyhow::bail!(UNEXPECTED_SPAWN_DETAIL)
            },
            &boundary,
            output_progress(),
            UnreleasedBootstrapGate,
        )
    });

    let error = result.err().unwrap();
    assert!(error.is::<process::DestructiveSessionUnavailable>());
    assert_eq!(spawn_calls.load(Ordering::Acquire), NO_SPAWN_CALLS);
    cleanup_scope.settle().await.unwrap();
}

#[tokio::test]
async fn available_atomic_signal_capability_reaches_spawn_boundary() {
    let spawn_calls = Arc::new(AtomicUsize::default());
    let callback_calls = Arc::clone(&spawn_calls);
    let mut cleanup_scope = StartupCleanupScope::new().unwrap();
    let spawn_guard = process::waitable_child_spawn_guard().unwrap();
    let boundary = spawn_guard.boundary();

    let result = process::with_test_signal_capability(true, || {
        spawn_with_reserved_reaper(
            &mut cleanup_scope,
            move || {
                callback_calls.fetch_add(SPAWN_CALL_INCREMENT, Ordering::AcqRel);
                anyhow::bail!(INJECTED_SPAWN_FAILURE)
            },
            &boundary,
            output_progress(),
            UnreleasedBootstrapGate,
        )
    });

    assert!(
        result
            .err()
            .unwrap()
            .to_string()
            .contains(INJECTED_SPAWN_FAILURE)
    );
    assert_eq!(spawn_calls.load(Ordering::Acquire), SINGLE_SPAWN_CALL);
    cleanup_scope.settle().await.unwrap();
}

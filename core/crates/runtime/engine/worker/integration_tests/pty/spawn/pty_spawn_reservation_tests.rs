use super::*;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

const NO_SPAWN_CALLS: usize = 0;
const SPAWN_CALL_INCREMENT: usize = 1;
const UNEXPECTED_SPAWN_DETAIL: &str = "spawn callback ran before reaper reservation succeeded";
const INJECTED_BIND_FAILURE: &str = "injected spawn binding failure";
const INJECTED_BIND_PANIC: &str = "injected spawn binding panic";
const INJECTED_GUARD_DROP_PANIC: &str = "injected spawn guard Drop panic";
const INJECTED_PAYLOAD_DROP_PANIC: &str = "injected spawn panic payload Drop panic";

struct ObservedSpawnGuard(Arc<AtomicBool>);

struct PanicSpawnGuard(Arc<AtomicBool>);

struct PayloadPanicSpawnGuard {
    released: Arc<AtomicBool>,
    payload_dropped: Arc<AtomicBool>,
}

struct PanicDropPayload(Arc<AtomicBool>);

impl Drop for ObservedSpawnGuard {
    fn drop(&mut self) {
        self.0.store(true, Ordering::Release);
    }
}

impl Drop for PanicSpawnGuard {
    fn drop(&mut self) {
        self.0.store(true, Ordering::Release);
        panic!("{INJECTED_GUARD_DROP_PANIC}");
    }
}

impl Drop for PayloadPanicSpawnGuard {
    fn drop(&mut self) {
        self.released.store(true, Ordering::Release);
        std::panic::panic_any(PanicDropPayload(Arc::clone(&self.payload_dropped)));
    }
}

impl Drop for PanicDropPayload {
    fn drop(&mut self) {
        self.0.store(true, Ordering::Release);
        panic!("{INJECTED_PAYLOAD_DROP_PANIC}");
    }
}

#[test]
fn reaper_reservation_failure_prevents_child_spawn() {
    let spawn_calls = Arc::new(AtomicUsize::new(usize::default()));
    let callback_calls = Arc::clone(&spawn_calls);
    let mut cleanup_scope = StartupCleanupScope::new().unwrap();
    let spawn_guard = process::waitable_child_spawn_guard().unwrap();
    let boundary = spawn_guard.boundary();
    startup_untracked_reaper::StartupUntrackedReaperReservation::fail_next_reservation();

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

#[test]
fn fault_reaper_reservation_failure_prevents_child_spawn() {
    let spawn_calls = Arc::new(AtomicUsize::new(usize::default()));
    let callback_calls = Arc::clone(&spawn_calls);
    let mut cleanup_scope = StartupCleanupScope::new().unwrap();
    let spawn_guard = process::waitable_child_spawn_guard().unwrap();
    let boundary = spawn_guard.boundary();
    pty_fault_reaper::FaultReaperReservation::fail_next_reservation();

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

#[test]
fn binding_error_releases_spawn_guard_before_propagation() {
    let released = Arc::new(AtomicBool::new(false));
    let result: anyhow::Result<()> =
        bind_before_releasing_spawn_guard(ObservedSpawnGuard(Arc::clone(&released)), |_| {
            anyhow::bail!(INJECTED_BIND_FAILURE)
        });

    assert!(result.is_err());
    assert!(released.load(Ordering::Acquire));
}

#[test]
fn binding_panic_releases_spawn_guard_before_resuming_unwind() {
    let released = Arc::new(AtomicBool::new(false));
    let panic = std::panic::catch_unwind({
        let released = Arc::clone(&released);
        move || {
            let _result: anyhow::Result<()> =
                bind_before_releasing_spawn_guard(ObservedSpawnGuard(released), |_| {
                    panic!("{INJECTED_BIND_PANIC}")
                });
        }
    });

    assert!(panic.is_err());
    assert!(released.load(Ordering::Acquire));
}

#[test]
fn spawn_guard_drop_panic_after_binding_success_becomes_an_error() {
    let released = Arc::new(AtomicBool::default());
    let result: anyhow::Result<()> =
        bind_before_releasing_spawn_guard(PanicSpawnGuard(Arc::clone(&released)), |_| Ok(()));

    assert!(result.is_err());
    assert!(released.load(Ordering::Acquire));
    assert!(
        result
            .unwrap_err()
            .to_string()
            .contains(SPAWN_GUARD_RELEASE_PANIC_DETAIL)
    );
}

#[test]
fn spawn_guard_drop_panic_cannot_replace_the_binding_panic() {
    let released = Arc::new(AtomicBool::default());
    let outcome = std::panic::catch_unwind({
        let released = Arc::clone(&released);
        move || {
            let _result: anyhow::Result<()> =
                bind_before_releasing_spawn_guard(PanicSpawnGuard(released), |_| {
                    panic!("{INJECTED_BIND_PANIC}")
                });
        }
    });

    let payload = outcome.unwrap_err();
    let detail = payload
        .downcast_ref::<&str>()
        .copied()
        .or_else(|| payload.downcast_ref::<String>().map(String::as_str));
    assert_eq!(detail, Some(INJECTED_BIND_PANIC));
    assert!(released.load(Ordering::Acquire));
}

#[test]
fn binding_and_guard_payload_drop_panics_preserve_the_original_unwind() {
    let released = Arc::new(AtomicBool::default());
    let guard_payload_dropped = Arc::new(AtomicBool::default());
    let binding_payload_dropped = Arc::new(AtomicBool::default());
    let outcome = std::panic::catch_unwind({
        let binding_payload_dropped = Arc::clone(&binding_payload_dropped);
        let guard = PayloadPanicSpawnGuard {
            released: Arc::clone(&released),
            payload_dropped: Arc::clone(&guard_payload_dropped),
        };
        move || {
            let _result: anyhow::Result<()> = bind_before_releasing_spawn_guard(guard, |_| {
                std::panic::panic_any(PanicDropPayload(binding_payload_dropped));
            });
        }
    });

    assert!(released.load(Ordering::Acquire));
    assert!(guard_payload_dropped.load(Ordering::Acquire));
    let _ = supervision::panic_detail(outcome.unwrap_err());
    assert!(binding_payload_dropped.load(Ordering::Acquire));
}

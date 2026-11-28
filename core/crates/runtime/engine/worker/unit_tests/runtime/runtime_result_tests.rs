use super::*;
use crate::worker::startup::startup_untracked_reaper::StartupUntrackedReaperReservation;

const INJECTED_RUNTIME_FAILURE: &str = "injected worker runtime failure";
const REAPER_PANIC_DETAIL: &str = "startup cleanup reaper thread panicked";

fn panicking_cleanup_scope() -> StartupCleanupScope {
    let mut scope = StartupCleanupScope::new().unwrap();
    let reservation = StartupUntrackedReaperReservation::reserve_panicking(&mut scope).unwrap();
    drop(reservation);
    scope
}

#[tokio::test]
async fn cleanup_failure_is_returned_after_a_successful_runtime() {
    let error = settle_runtime_result(Ok(()), panicking_cleanup_scope())
        .await
        .unwrap_err();

    assert!(format!("{error:#}").contains(REAPER_PANIC_DETAIL));
}

#[tokio::test]
async fn cleanup_failure_retains_the_original_runtime_context() {
    let runtime = anyhow::anyhow!(INJECTED_RUNTIME_FAILURE);
    let error = settle_runtime_result(Err(runtime), panicking_cleanup_scope())
        .await
        .unwrap_err();
    let detail = format!("{error:#}");

    assert!(detail.contains(INJECTED_RUNTIME_FAILURE));
    assert!(detail.contains(REAPER_PANIC_DETAIL));
}

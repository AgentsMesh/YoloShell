use super::*;
use std::sync::atomic::{AtomicBool, Ordering};

const RESERVATION_FAILURE: &str = "injected guardian reaper reservation failure";
const SPAWN_FAILURE: &str = "guardian spawn ran after reservation failure";
const UNEXPECTED_SUCCESS: &str = "guardian spawn unexpectedly succeeded";

#[test]
fn reaper_reservation_failure_precedes_guardian_spawn() {
    let spawn_called = AtomicBool::new(false);

    let result = reserve_then_spawn(
        || Err(anyhow::anyhow!(RESERVATION_FAILURE)),
        || {
            spawn_called.store(true, Ordering::SeqCst);
            Err(anyhow::anyhow!(SPAWN_FAILURE))
        },
    );

    let error = match result {
        Ok(_) => panic!("{UNEXPECTED_SUCCESS}"),
        Err(error) => error,
    };
    assert_eq!(error.to_string(), RESERVATION_FAILURE);
    assert!(!spawn_called.load(Ordering::SeqCst));
}

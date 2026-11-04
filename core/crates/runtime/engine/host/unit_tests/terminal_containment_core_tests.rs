use super::*;
use std::fmt;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

const ERROR_DISPLAY_PANIC: &str = "injected host error Display panic";
const ERROR_DROP_PANIC: &str = "injected host error Drop panic";
const PAYLOAD_DROP_PANIC: &str = "injected host payload Drop panic";

#[derive(Debug)]
struct PanicError(Arc<AtomicBool>);

impl fmt::Display for PanicError {
    fn fmt(&self, _formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        panic!("{ERROR_DISPLAY_PANIC}");
    }
}

impl std::error::Error for PanicError {}

impl Drop for PanicError {
    fn drop(&mut self) {
        self.0.store(true, Ordering::Release);
        panic!("{ERROR_DROP_PANIC}");
    }
}

struct PanicPayload(Arc<AtomicBool>);

impl Drop for PanicPayload {
    fn drop(&mut self) {
        self.0.store(true, Ordering::Release);
        panic!("{PAYLOAD_DROP_PANIC}");
    }
}

#[test]
fn error_display_and_drop_panics_are_contained() {
    let dropped = Arc::new(AtomicBool::default());
    let observed = Arc::clone(&dropped);
    let outcome = catch_unwind(AssertUnwindSafe(|| {
        error_detail(anyhow::Error::new(PanicError(dropped)))
    }));

    assert_eq!(outcome.unwrap(), ERROR_DETAIL_FALLBACK);
    assert!(observed.load(Ordering::Acquire));
}

#[tokio::test]
async fn future_payload_drop_panic_is_contained_with_a_stable_detail() {
    let dropped = Arc::new(AtomicBool::default());
    let observed = Arc::clone(&dropped);
    let result = catch_future_detail(async move {
        std::panic::panic_any(PanicPayload(dropped));
    })
    .await;

    assert_eq!(result.unwrap_err(), PANIC_DETAIL_FALLBACK);
    assert!(observed.load(Ordering::Acquire));
}

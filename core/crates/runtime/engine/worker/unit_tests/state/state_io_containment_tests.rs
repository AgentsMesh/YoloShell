use super::*;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

const OUTER_PANIC: &str = "injected input delivery outer unwind";
const DISPLAY_PANIC: &str = "injected input delivery error Display panic";
const PAYLOAD_DROP_PANIC: &str = "injected input delivery payload Drop panic";

#[derive(Debug)]
struct HostileIoError {
    error_dropped: Arc<AtomicBool>,
    payload_dropped: Arc<AtomicBool>,
}

impl std::fmt::Display for HostileIoError {
    fn fmt(&self, _formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        panic!("{DISPLAY_PANIC}");
    }
}

impl std::error::Error for HostileIoError {}

impl Drop for HostileIoError {
    fn drop(&mut self) {
        self.error_dropped.store(true, Ordering::SeqCst);
        std::panic::panic_any(DropPayload(Arc::clone(&self.payload_dropped)));
    }
}

struct DropPayload(Arc<AtomicBool>);

impl Drop for DropPayload {
    fn drop(&mut self) {
        self.0.store(true, Ordering::SeqCst);
        panic!("{PAYLOAD_DROP_PANIC}");
    }
}

struct RenderOnDrop {
    error: Option<io::Error>,
}

impl Drop for RenderOnDrop {
    fn drop(&mut self) {
        let _ = input_delivery_unknown(
            Uuid::nil(),
            Some(self.error.take().expect("hostile IO error is present")),
        );
    }
}

#[test]
fn input_delivery_unknown_contains_error_display_and_drop_during_outer_unwind() {
    let error_dropped = Arc::new(AtomicBool::default());
    let payload_dropped = Arc::new(AtomicBool::default());
    let guard = RenderOnDrop {
        error: Some(io::Error::other(HostileIoError {
            error_dropped: Arc::clone(&error_dropped),
            payload_dropped: Arc::clone(&payload_dropped),
        })),
    };

    let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let _guard = guard;
        panic!("{OUTER_PANIC}");
    }));

    assert_eq!(panic_message(outcome.unwrap_err().as_ref()), Some(OUTER_PANIC));
    assert!(error_dropped.load(Ordering::SeqCst));
    assert!(!payload_dropped.load(Ordering::SeqCst));
}

fn panic_message(payload: &(dyn std::any::Any + Send)) -> Option<&str> {
    payload
        .downcast_ref::<&str>()
        .copied()
        .or_else(|| payload.downcast_ref::<String>().map(String::as_str))
}

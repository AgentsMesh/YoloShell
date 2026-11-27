use super::*;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use tracing::span::{Attributes, Id, Record};
use tracing::{Event, Metadata, Subscriber};

const OUTER_PANIC: &str = "injected untracked driver outer unwind";
const ERROR_DISPLAY_PANIC: &str = "injected untracked driver error Display panic";
const ERROR_SOURCE_PANIC: &str = "injected untracked driver error source panic";
const PAYLOAD_DROP_PANIC: &str = "injected untracked driver payload Drop panic";
const TEST_SPAN_ID: u64 = 1;

#[derive(Debug)]
struct HostileError {
    error_dropped: Arc<AtomicBool>,
    payload_dropped: Arc<AtomicBool>,
}

impl std::fmt::Display for HostileError {
    fn fmt(&self, _formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        panic!("{ERROR_DISPLAY_PANIC}");
    }
}

impl std::error::Error for HostileError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        panic!("{ERROR_SOURCE_PANIC}");
    }
}

impl Drop for HostileError {
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

struct PanicSubscriber(Arc<AtomicUsize>);

impl Subscriber for PanicSubscriber {
    fn enabled(&self, _metadata: &Metadata<'_>) -> bool {
        true
    }

    fn new_span(&self, _attributes: &Attributes<'_>) -> Id {
        Id::from_u64(TEST_SPAN_ID)
    }

    fn record(&self, _span: &Id, _values: &Record<'_>) {}
    fn record_follows_from(&self, _span: &Id, _follows: &Id) {}

    fn event(&self, _event: &Event<'_>) {
        self.0.fetch_add(1, Ordering::SeqCst);
        panic!("injected untracked driver tracing panic");
    }

    fn enter(&self, _span: &Id) {}
    fn exit(&self, _span: &Id) {}
}

struct ClassifyOnDrop {
    error: Option<anyhow::Error>,
}

impl Drop for ClassifyOnDrop {
    fn drop(&mut self) {
        let mut authority_lost = false;
        child_exited(
            Some(Err(self.error.take().expect("hostile error is present"))),
            &mut authority_lost,
        );
    }
}

#[test]
fn driver_error_source_display_drop_and_trace_panics_remain_contained_during_outer_unwind() {
    let error_dropped = Arc::new(AtomicBool::default());
    let payload_dropped = Arc::new(AtomicBool::default());
    let trace_events = Arc::new(AtomicUsize::default());
    let guard = ClassifyOnDrop {
        error: Some(anyhow::Error::new(HostileError {
            error_dropped: Arc::clone(&error_dropped),
            payload_dropped: Arc::clone(&payload_dropped),
        })),
    };

    let _tracing_test = crate::tracing_test_support::serialize_subscriber_tests();
    let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        tracing::subscriber::with_default(PanicSubscriber(Arc::clone(&trace_events)), || {
            let _guard = guard;
            panic!("{OUTER_PANIC}");
        });
    }));

    assert_eq!(
        panic_message(outcome.unwrap_err().as_ref()),
        Some(OUTER_PANIC)
    );
    assert!(error_dropped.load(Ordering::SeqCst));
    assert!(!payload_dropped.load(Ordering::SeqCst));
    assert_eq!(trace_events.load(Ordering::SeqCst), 1);
}

fn panic_message(payload: &(dyn std::any::Any + Send)) -> Option<&str> {
    payload
        .downcast_ref::<&str>()
        .copied()
        .or_else(|| payload.downcast_ref::<String>().map(String::as_str))
}

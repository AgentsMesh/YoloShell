use std::panic::{AssertUnwindSafe, catch_unwind};
use tracing::span::{Attributes, Id, Record};
use tracing::{Event, Metadata, Subscriber};

const CLEANUP_ERROR_DETAIL: &str = "startup cleanup error for tracing containment";
const OUTER_PANIC_DETAIL: &str = "injected outer unwind around tracing containment";
const SUBSCRIBER_PANIC_DETAIL: &str = "injected tracing subscriber panic";
const TEST_SPAN_ID: u64 = 1;

struct PanicSubscriber;
struct CleanupBoundary;

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
        panic!("{SUBSCRIBER_PANIC_DETAIL}");
    }

    fn enter(&self, _span: &Id) {}

    fn exit(&self, _span: &Id) {}
}

impl Drop for CleanupBoundary {
    fn drop(&mut self) {
        super::contain_drop_cleanup(|| anyhow::bail!(CLEANUP_ERROR_DETAIL));
    }
}

#[test]
fn tracing_subscriber_panic_is_contained_during_outer_unwind() {
    let _tracing_test = crate::tracing_test_support::serialize_subscriber_tests();
    let outcome = catch_unwind(AssertUnwindSafe(|| {
        tracing::subscriber::with_default(PanicSubscriber, || {
            let _boundary = CleanupBoundary;
            panic!("{OUTER_PANIC_DETAIL}");
        });
    }));

    let payload = outcome.unwrap_err();
    let detail = payload
        .downcast_ref::<&str>()
        .copied()
        .or_else(|| payload.downcast_ref::<String>().map(String::as_str));
    assert_eq!(detail, Some(OUTER_PANIC_DETAIL));
}

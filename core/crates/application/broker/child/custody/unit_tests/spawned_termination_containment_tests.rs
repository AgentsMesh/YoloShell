use super::containment;
use std::fmt;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use tracing::span::{Attributes, Id, Record};
use tracing::{Event, Metadata, Subscriber};

const ERROR_DISPLAY_PANIC: &str = "injected broker cleanup error Display panic";
const ERROR_DROP_PANIC: &str = "injected broker cleanup error Drop panic";
const OUTER_PANIC: &str = "injected broker cleanup outer panic";
const PANIC_PAYLOAD_DROP: &str = "injected broker cleanup payload Drop panic";
const REPORT_MESSAGE: &str = "broker cleanup containment test report";
const SUBSCRIBER_PANIC: &str = "injected broker cleanup subscriber panic";
const TEST_PROCESS_ID: u32 = 41;
const TEST_SPAN_ID: u64 = 1;

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

struct PanicOnDropPayload;

impl Drop for PanicOnDropPayload {
    fn drop(&mut self) {
        panic!("{PANIC_PAYLOAD_DROP}");
    }
}

struct PanicSubscriber;

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
        panic!("{SUBSCRIBER_PANIC}");
    }

    fn enter(&self, _span: &Id) {}

    fn exit(&self, _span: &Id) {}
}

struct PayloadBoundary;

impl Drop for PayloadBoundary {
    fn drop(&mut self) {
        let _ = containment::catch_unwind(|| std::panic::panic_any(PanicOnDropPayload));
    }
}

struct ReportBoundary(Arc<AtomicBool>);

impl Drop for ReportBoundary {
    fn drop(&mut self) {
        let error = std::io::Error::other(PanicError(Arc::clone(&self.0)));
        containment::report(error, TEST_PROCESS_ID, REPORT_MESSAGE);
    }
}

#[test]
fn spawned_reaper_panic_payload_drop_is_contained_during_outer_unwind() {
    let outcome = catch_unwind(AssertUnwindSafe(|| {
        let _boundary = PayloadBoundary;
        panic!("{OUTER_PANIC}");
    }));

    assert_primary_panic(outcome);
}

#[test]
fn spawned_reaper_reporting_panics_are_contained_during_outer_unwind() {
    let dropped = Arc::new(AtomicBool::new(false));
    let boundary_dropped = Arc::clone(&dropped);
    let _tracing_test = crate::tracing_test_support::serialize_subscriber_tests();
    let outcome = catch_unwind(AssertUnwindSafe(|| {
        tracing::subscriber::with_default(PanicSubscriber, || {
            let _boundary = ReportBoundary(boundary_dropped);
            panic!("{OUTER_PANIC}");
        });
    }));

    assert_primary_panic(outcome);
    assert!(dropped.load(Ordering::Acquire));
}

#[test]
fn spawned_reaper_error_detail_preserves_reportable_errors() {
    let detail = containment::error_detail(std::io::Error::other(REPORT_MESSAGE));

    assert_eq!(detail, REPORT_MESSAGE);
}

fn assert_primary_panic(outcome: std::thread::Result<()>) {
    let payload = outcome.unwrap_err();
    let detail = payload
        .downcast_ref::<&str>()
        .copied()
        .or_else(|| payload.downcast_ref::<String>().map(String::as_str));
    assert_eq!(detail, Some(OUTER_PANIC));
}

use super::*;
use std::fmt;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use tracing::span::{Attributes, Id, Record};
use tracing::{Event, Metadata, Subscriber};

const ERROR_DISPLAY_PANIC: &str = "injected terminal report error Display panic";
const ERROR_DROP_PANIC: &str = "injected terminal report error Drop panic";
const INNER_ERROR_DETAIL: &str = "terminal report inner error";
const OUTER_ERROR_CONTEXT: &str = "terminal report outer context";
const OUTER_PANIC: &str = "injected terminal report outer panic";
const REPORT_DETAIL: &str = "terminal report detail";
const REPORT_MESSAGE: &str = "terminal report message";
const SUBSCRIBER_PANIC: &str = "injected terminal report subscriber panic";
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

struct ReportingBoundary(Arc<AtomicBool>);

impl Drop for ReportingBoundary {
    fn drop(&mut self) {
        warn_error(
            anyhow::Error::new(PanicError(Arc::clone(&self.0))),
            REPORT_MESSAGE,
        );
        warn_detail(REPORT_DETAIL, REPORT_MESSAGE);
        warn_message(REPORT_MESSAGE);
        error_detail(REPORT_DETAIL, REPORT_MESSAGE);
        error_error(
            anyhow::Error::new(PanicError(Arc::clone(&self.0))),
            REPORT_MESSAGE,
        );
        error_message(REPORT_MESSAGE);
        debug_message(REPORT_MESSAGE);
        discard_error(anyhow::Error::new(PanicError(Arc::clone(&self.0))));
    }
}

#[test]
fn referenced_error_text_preserves_the_error_chain() {
    let error = anyhow::anyhow!(INNER_ERROR_DETAIL).context(OUTER_ERROR_CONTEXT);
    let detail = error_ref_text(&error);

    assert!(detail.contains(OUTER_ERROR_CONTEXT));
    assert!(detail.contains(INNER_ERROR_DETAIL));
}

#[test]
fn reports_contain_error_and_subscriber_panics_during_outer_unwind() {
    let dropped = Arc::new(AtomicBool::default());
    let observed = Arc::clone(&dropped);
    let _tracing_test = crate::tracing_test_support::serialize_subscriber_tests();
    let outcome = catch_unwind(AssertUnwindSafe(|| {
        tracing::subscriber::with_default(PanicSubscriber, || {
            let _boundary = ReportingBoundary(dropped);
            panic!("{OUTER_PANIC}");
        });
    }));

    assert!(outcome.is_err());
    assert!(observed.load(Ordering::Acquire));
}

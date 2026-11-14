use super::*;
use std::fmt;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use tracing::span::{Attributes, Id, Record};
use tracing::{Event, Metadata, Subscriber};

const ERROR_DISPLAY_PANIC: &str = "injected broker task error Display panic";
const ERROR_DROP_PANIC: &str = "injected broker task error Drop panic";
const OUTER_PANIC: &str = "injected broker task reporting outer panic";
const REPORT_MESSAGE: &str = "broker task containment test report";
const SUBSCRIBER_PANIC: &str = "injected broker task subscriber panic";
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

struct ReportBoundary(Arc<AtomicBool>);

impl Drop for ReportBoundary {
    fn drop(&mut self) {
        let error = anyhow::Error::new(PanicError(Arc::clone(&self.0)));
        report_task_error(error, REPORT_MESSAGE);
    }
}

#[test]
fn broker_task_reporting_panics_are_contained_during_outer_unwind() {
    let dropped = Arc::new(AtomicBool::new(false));
    let boundary_dropped = Arc::clone(&dropped);
    let _tracing_test = crate::tracing_test_support::serialize_subscriber_tests();
    let outcome = catch_unwind(AssertUnwindSafe(|| {
        tracing::subscriber::with_default(PanicSubscriber, || {
            let _boundary = ReportBoundary(boundary_dropped);
            panic!("{OUTER_PANIC}");
        });
    }));

    let payload = outcome.unwrap_err();
    let detail = payload
        .downcast_ref::<&str>()
        .copied()
        .or_else(|| payload.downcast_ref::<String>().map(String::as_str));
    assert_eq!(detail, Some(OUTER_PANIC));
    assert!(dropped.load(Ordering::Acquire));
}

#[test]
fn broker_task_error_detail_preserves_reportable_errors() {
    let detail = task_error_detail(anyhow::Error::msg(REPORT_MESSAGE));

    assert_eq!(detail, REPORT_MESSAGE);
}

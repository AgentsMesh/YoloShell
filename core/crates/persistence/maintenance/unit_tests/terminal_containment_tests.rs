use super::*;
use anyhow::Result;
use std::fmt;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use tracing::span::{Attributes, Id, Record};
use tracing::{Event, Metadata, Subscriber};

const ERROR_DISPLAY_PANIC: &str = "injected scanner error Display panic";
const ERROR_DROP_PANIC: &str = "injected scanner error Drop panic";
const OUTER_PANIC: &str = "injected outer scanner unwind";
const PAYLOAD_DROP_PANIC: &str = "injected scanner payload Drop panic";
const SCANNER_LABEL: &str = "terminal-containment-scanner";
const SUBSCRIBER_PANIC: &str = "injected scanner tracing subscriber panic";
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

struct ErrorScanner {
    control: Arc<control::StopControl>,
    error_dropped: Arc<AtomicBool>,
}

impl MaintenanceScanner for ErrorScanner {
    fn label(&self) -> &'static str {
        SCANNER_LABEL
    }

    fn scan_chunk(&mut self) -> Result<ScanChunk> {
        Err(anyhow::Error::new(PanicError(Arc::clone(
            &self.error_dropped,
        ))))
    }

    fn reset(&mut self) {
        self.control.request();
    }
}

struct PanicPayload(Arc<AtomicBool>);

impl Drop for PanicPayload {
    fn drop(&mut self) {
        self.0.store(true, Ordering::Release);
        panic!("{PAYLOAD_DROP_PANIC}");
    }
}

struct PanicScanner {
    control: Arc<control::StopControl>,
    payload_dropped: Arc<AtomicBool>,
}

impl MaintenanceScanner for PanicScanner {
    fn label(&self) -> &'static str {
        SCANNER_LABEL
    }

    fn scan_chunk(&mut self) -> Result<ScanChunk> {
        std::panic::panic_any(PanicPayload(Arc::clone(&self.payload_dropped)))
    }

    fn reset(&mut self) {
        self.control.request();
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

struct ScannerBoundary {
    control: Arc<control::StopControl>,
    payload_dropped: Arc<AtomicBool>,
}

impl Drop for ScannerBoundary {
    fn drop(&mut self) {
        let mut scanners: ScannerSet = vec![Box::new(PanicScanner {
            control: Arc::clone(&self.control),
            payload_dropped: Arc::clone(&self.payload_dropped),
        })];
        scanner_loop::run_as_owner(&mut scanners, &self.control, &AtomicBool::default());
    }
}

#[test]
fn scanner_error_display_and_drop_panics_are_contained() {
    let control = Arc::new(control::StopControl::new());
    let error_dropped = Arc::new(AtomicBool::default());
    let mut scanners: ScannerSet = vec![Box::new(ErrorScanner {
        control: Arc::clone(&control),
        error_dropped: Arc::clone(&error_dropped),
    })];

    let outcome = catch_unwind(AssertUnwindSafe(|| {
        scanner_loop::run_as_owner(&mut scanners, &control, &AtomicBool::default());
    }));

    assert!(outcome.is_ok());
    assert!(error_dropped.load(Ordering::Acquire));
    assert!(scanners.is_empty());
}

#[test]
fn scanner_payload_and_subscriber_panics_are_contained_during_outer_unwind() {
    let control = Arc::new(control::StopControl::new());
    let payload_dropped = Arc::new(AtomicBool::default());
    let observed = Arc::clone(&payload_dropped);
    let _tracing_test = yolo_application_test_fixtures::tracing::serialize_subscriber_tests();
    let outcome = catch_unwind(AssertUnwindSafe(|| {
        tracing::subscriber::with_default(PanicSubscriber, || {
            let _boundary = ScannerBoundary {
                control,
                payload_dropped,
            };
            panic!("{OUTER_PANIC}");
        });
    }));

    let payload = outcome.unwrap_err();
    let detail = payload
        .downcast_ref::<&str>()
        .copied()
        .or_else(|| payload.downcast_ref::<String>().map(String::as_str));
    assert_eq!(detail, Some(OUTER_PANIC));
    assert!(observed.load(Ordering::Acquire));
}

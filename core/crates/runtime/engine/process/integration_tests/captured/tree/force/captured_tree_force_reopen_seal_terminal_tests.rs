use super::*;
use std::fmt;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use tracing::span::{Attributes, Id, Record};
use tracing::{Event, Metadata, Subscriber};

const DISPLAY_PANIC: &str = "injected reopen-seal cleanup error Display panic";
const DROP_PANIC: &str = "injected reopen-seal cleanup error Drop panic";
const MESSAGE: &str = "reopen-seal terminal containment test";
const PAYLOAD_DROP_PANIC: &str = "injected reopen-seal subscriber payload Drop panic";
const TEST_SPAN_ID: u64 = 1;

#[derive(Debug)]
struct AdversarialError {
    displays: Arc<AtomicUsize>,
    drops: Arc<AtomicUsize>,
}

impl fmt::Display for AdversarialError {
    fn fmt(&self, _formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.displays.fetch_add(1, Ordering::SeqCst);
        panic!("{DISPLAY_PANIC}");
    }
}

impl std::error::Error for AdversarialError {}

impl Drop for AdversarialError {
    fn drop(&mut self) {
        self.drops.fetch_add(1, Ordering::SeqCst);
        panic!("{DROP_PANIC}");
    }
}

struct PanicSubscriber {
    events: Arc<AtomicUsize>,
    payload_drops: Arc<AtomicUsize>,
}

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
        self.events.fetch_add(1, Ordering::SeqCst);
        std::panic::panic_any(PanicPayload(Arc::clone(&self.payload_drops)));
    }

    fn enter(&self, _span: &Id) {}
    fn exit(&self, _span: &Id) {}
}

struct PanicPayload(Arc<AtomicUsize>);

impl Drop for PanicPayload {
    fn drop(&mut self) {
        self.0.fetch_add(1, Ordering::SeqCst);
        panic!("{PAYLOAD_DROP_PANIC}");
    }
}

#[test]
fn replacement_and_success_discard_contain_error_drop_panics() {
    let displays = Arc::new(AtomicUsize::default());
    let drops = Arc::new(AtomicUsize::default());
    let mut failure = None;

    remember_failure(&mut failure, adversarial_error(&displays, &drops));
    remember_failure(&mut failure, adversarial_error(&displays, &drops));
    discard_failure(&mut failure);

    assert!(failure.is_none());
    assert_eq!(displays.load(Ordering::SeqCst), 0);
    assert_eq!(drops.load(Ordering::SeqCst), 2);
}

#[test]
fn report_contains_format_drop_subscriber_and_payload_panics() {
    let displays = Arc::new(AtomicUsize::default());
    let drops = Arc::new(AtomicUsize::default());
    let events = Arc::new(AtomicUsize::default());
    let payload_drops = Arc::new(AtomicUsize::default());
    let subscriber = PanicSubscriber {
        events: Arc::clone(&events),
        payload_drops: Arc::clone(&payload_drops),
    };
    let _tracing_test = yolo_application_test_fixtures::tracing::serialize_subscriber_tests();

    let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        tracing::subscriber::with_default(subscriber, || {
            report_failure(adversarial_error(&displays, &drops), MESSAGE);
        });
    }));

    assert!(outcome.is_ok());
    assert_eq!(displays.load(Ordering::SeqCst), 1);
    assert_eq!(drops.load(Ordering::SeqCst), 1);
    assert_eq!(events.load(Ordering::SeqCst), 1);
    assert_eq!(payload_drops.load(Ordering::SeqCst), 1);
}

fn adversarial_error(displays: &Arc<AtomicUsize>, drops: &Arc<AtomicUsize>) -> Error {
    Error::new(AdversarialError {
        displays: Arc::clone(displays),
        drops: Arc::clone(drops),
    })
}

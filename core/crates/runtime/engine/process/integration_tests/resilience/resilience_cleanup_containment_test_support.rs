use std::fmt;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use tracing::span::{Attributes, Id, Record};
use tracing::{Event, Metadata, Subscriber};

const ERROR_DISPLAY_PANIC: &str = "injected resilience cleanup error Display panic";
const ERROR_DROP_PANIC: &str = "injected resilience cleanup error Drop panic";
const PAYLOAD_DROP_PANIC: &str = "injected resilience cleanup trace payload Drop panic";
const TEST_SPAN_ID: u64 = 1;

#[derive(Debug)]
struct AdversarialError {
    displays: Arc<AtomicUsize>,
    drops: Arc<AtomicUsize>,
}

impl fmt::Display for AdversarialError {
    fn fmt(&self, _formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.displays.fetch_add(1, Ordering::SeqCst);
        panic!("{ERROR_DISPLAY_PANIC}");
    }
}

impl std::error::Error for AdversarialError {}

impl Drop for AdversarialError {
    fn drop(&mut self) {
        self.drops.fetch_add(1, Ordering::SeqCst);
        panic!("{ERROR_DROP_PANIC}");
    }
}

pub(super) struct PanicSubscriber {
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

pub(super) fn adversarial_error(
    displays: &Arc<AtomicUsize>,
    drops: &Arc<AtomicUsize>,
) -> anyhow::Error {
    anyhow::Error::new(AdversarialError {
        displays: Arc::clone(displays),
        drops: Arc::clone(drops),
    })
}

pub(super) fn panic_subscriber() -> (PanicSubscriber, Arc<AtomicUsize>, Arc<AtomicUsize>) {
    let events = Arc::new(AtomicUsize::default());
    let payload_drops = Arc::new(AtomicUsize::default());
    (
        PanicSubscriber {
            events: Arc::clone(&events),
            payload_drops: Arc::clone(&payload_drops),
        },
        events,
        payload_drops,
    )
}

pub(super) fn panic_with_payload(drops: &Arc<AtomicUsize>) -> ! {
    std::panic::panic_any(PanicPayload(Arc::clone(drops)))
}

pub(super) fn panic_message(payload: &(dyn std::any::Any + Send)) -> Option<&str> {
    payload
        .downcast_ref::<&str>()
        .copied()
        .or_else(|| payload.downcast_ref::<String>().map(String::as_str))
}

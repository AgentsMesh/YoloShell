use super::*;
use std::fmt;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use tracing::span::{Attributes, Id, Record};
use tracing::{Event, Metadata, Subscriber};

const DISPLAY_PANIC: &str = "injected child reaper error Display panic";
const DROP_PANIC: &str = "injected child reaper error Drop panic";
const OUTER_PANIC: &str = "injected outer child reaper unwind";
const TRACE_PAYLOAD_DROP_PANIC: &str = "injected child reaper trace payload Drop panic";
const TEST_ERROR_DETAIL: &str = "injected child reaper poll failure";
const TEST_PROCESS_ID: u32 = 7_100;
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

struct ErrorChild(Option<io::Error>);

impl ReapChild for ErrorChild {
    fn process_id(&self) -> u32 {
        TEST_PROCESS_ID
    }

    fn try_reap(&mut self) -> io::Result<bool> {
        Err(self.0.take().unwrap())
    }

    fn request_termination(&mut self) -> io::Result<()> {
        Ok(())
    }
}

#[test]
fn poll_error_display_and_drop_panics_remain_contained() {
    let displays = Arc::new(AtomicUsize::new(usize::default()));
    let drops = Arc::new(AtomicUsize::new(usize::default()));
    let error = AdversarialError {
        displays: Arc::clone(&displays),
        drops: Arc::clone(&drops),
    };
    let mut children = retained(io::Error::other(error));

    assert_eq!(reap_retained_with_progress(&mut children).settled, 0);
    assert_eq!(children.len(), 1);
    assert_eq!(displays.load(Ordering::SeqCst), 1);
    assert_eq!(drops.load(Ordering::SeqCst), 1);
}

struct TerminationErrorChild(Option<io::Error>);

impl ReapChild for TerminationErrorChild {
    fn process_id(&self) -> u32 {
        TEST_PROCESS_ID
    }

    fn try_reap(&mut self) -> io::Result<bool> {
        Ok(false)
    }

    fn request_termination(&mut self) -> io::Result<()> {
        Err(self.0.take().unwrap())
    }
}

#[test]
fn termination_error_display_and_drop_panics_remain_contained() {
    let displays = Arc::new(AtomicUsize::new(usize::default()));
    let drops = Arc::new(AtomicUsize::new(usize::default()));
    let error = AdversarialError {
        displays: Arc::clone(&displays),
        drops: Arc::clone(&drops),
    };
    let mut children = vec![RetainedChild {
        child: TerminationErrorChild(Some(io::Error::other(error))),
        policy: TerminationPolicy::Terminate,
    }];

    assert_eq!(reap_retained_with_progress(&mut children).settled, 0);
    assert_eq!(children[0].policy, TerminationPolicy::Terminate);
    assert_eq!(displays.load(Ordering::SeqCst), 1);
    assert_eq!(drops.load(Ordering::SeqCst), 1);
}

struct PanicSubscriber {
    events: Arc<AtomicUsize>,
    payload_dropped: Arc<AtomicBool>,
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
        std::panic::panic_any(TracePayload(Arc::clone(&self.payload_dropped)));
    }
    fn enter(&self, _span: &Id) {}
    fn exit(&self, _span: &Id) {}
}

struct TracePayload(Arc<AtomicBool>);

impl Drop for TracePayload {
    fn drop(&mut self) {
        self.0.store(true, Ordering::SeqCst);
        panic!("{TRACE_PAYLOAD_DROP_PANIC}");
    }
}

struct ReapDuringUnwind(Vec<RetainedChild<ErrorChild>>);

impl Drop for ReapDuringUnwind {
    fn drop(&mut self) {
        let _ = reap_retained_with_progress(&mut self.0);
    }
}

#[test]
fn trace_panic_payload_is_contained_during_outer_unwind() {
    let events = Arc::new(AtomicUsize::new(usize::default()));
    let payload_dropped = Arc::new(AtomicBool::new(false));
    let subscriber = PanicSubscriber {
        events: Arc::clone(&events),
        payload_dropped: Arc::clone(&payload_dropped),
    };
    let _tracing_test = yolo_application_test_fixtures::tracing::serialize_subscriber_tests();
    let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        tracing::subscriber::with_default(subscriber, || {
            let _boundary = ReapDuringUnwind(retained(io::Error::other(TEST_ERROR_DETAIL)));
            panic!("{OUTER_PANIC}");
        });
    }));

    let payload = outcome.unwrap_err();
    assert_eq!(panic_message(payload.as_ref()), Some(OUTER_PANIC));
    assert_eq!(events.load(Ordering::SeqCst), 1);
    assert!(payload_dropped.load(Ordering::SeqCst));
}

fn retained(error: io::Error) -> Vec<RetainedChild<ErrorChild>> {
    vec![RetainedChild {
        child: ErrorChild(Some(error)),
        policy: TerminationPolicy::NaturalExit,
    }]
}

fn panic_message(payload: &(dyn std::any::Any + Send)) -> Option<&str> {
    payload
        .downcast_ref::<&str>()
        .copied()
        .or_else(|| payload.downcast_ref::<String>().map(String::as_str))
}

use super::*;
use crate::process::{CapturedProcessTree, ProcessIdentity};
use std::path::Path;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::time::{Duration, Instant};
use tracing::span::{Attributes, Id, Record};
use tracing::{Event, Metadata, Subscriber};

const CHILD_EXECUTABLE: &str = "/bin/sleep";
const CHILD_DURATION: &str = "30";
const OUTER_PANIC_DETAIL: &str = "injected outer PTY ownership unwind";
const EMERGENCY_PAYLOAD_DROP_PANIC: &str = "injected PTY emergency payload Drop panic";
const SUBSCRIBER_PAYLOAD_DROP_PANIC: &str = "injected PTY ownership subscriber payload Drop panic";
const TEST_SPAN_ID: u64 = 1;
const WAIT_INTERVAL: Duration = Duration::from_millis(5);
const WAIT_TIMEOUT: Duration = Duration::from_secs(10);

#[test]
fn handoff_dispatch_panic_moves_live_custody_to_fallback() {
    let mut reaper = reserve_fault_reaper().unwrap();
    reaper.panic_on_handoff_dispatch_for_test();
    let child = crate::process::spawn_detached(
        Path::new(CHILD_EXECUTABLE),
        &[CHILD_DURATION.into()],
        crate::process::DetachedStandardError::Null,
    )
    .unwrap();
    let identity = ProcessIdentity::capture(child.id()).unwrap();
    let tree = CapturedProcessTree::capture(identity).unwrap();
    let mut ownership = PtyOwnership::new_for_test(Box::new(child), tree, reaper).unwrap();
    let (completion, receiver) = FaultReapCompletion::owner_channel();

    let settlement = ownership.settle_after_fault(completion);

    assert!(matches!(settlement, FaultSettlement::Deferred(_)));
    let outcome = receiver.receive_timeout(WAIT_TIMEOUT).unwrap();
    assert_eq!(outcome.evidence, ReapEvidence::Waited);
    assert!(outcome.status.is_some());
    wait_until_gone(identity);
}

#[test]
fn handoff_construction_panic_retries_without_losing_custody() {
    let mut reaper = reserve_fault_reaper().unwrap();
    reaper.panic_on_handoff_build_for_test();
    let child = crate::process::spawn_detached(
        Path::new(CHILD_EXECUTABLE),
        &[CHILD_DURATION.into()],
        crate::process::DetachedStandardError::Null,
    )
    .unwrap();
    let identity = ProcessIdentity::capture(child.id()).unwrap();
    let tree = CapturedProcessTree::capture(identity).unwrap();
    let mut ownership = PtyOwnership::new_for_test(Box::new(child), tree, reaper).unwrap();
    let (completion, receiver) = FaultReapCompletion::owner_channel();

    let settlement = ownership.settle_after_fault(completion);

    assert!(matches!(settlement, FaultSettlement::Deferred(_)));
    let outcome = receiver.receive_timeout(WAIT_TIMEOUT).unwrap();
    assert_eq!(outcome.evidence, ReapEvidence::Waited);
    assert!(outcome.status.is_some());
    wait_until_gone(identity);
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
        std::panic::panic_any(SubscriberPayload(Arc::clone(&self.payload_dropped)));
    }

    fn enter(&self, _span: &Id) {}
    fn exit(&self, _span: &Id) {}
}

struct SubscriberPayload(Arc<AtomicBool>);

impl Drop for SubscriberPayload {
    fn drop(&mut self) {
        self.0.store(true, Ordering::SeqCst);
        panic!("{SUBSCRIBER_PAYLOAD_DROP_PANIC}");
    }
}

struct EmergencyPayload(Arc<AtomicBool>);

impl Drop for EmergencyPayload {
    fn drop(&mut self) {
        self.0.store(true, Ordering::SeqCst);
        panic!("{EMERGENCY_PAYLOAD_DROP_PANIC}");
    }
}

#[test]
fn ownership_drop_defers_panicking_payload_without_entering_tracing_during_outer_unwind() {
    let child = crate::process::spawn_detached(
        Path::new(CHILD_EXECUTABLE),
        &[CHILD_DURATION.into()],
        crate::process::DetachedStandardError::Null,
    )
    .unwrap();
    let identity = ProcessIdentity::capture(child.id()).unwrap();
    let tree = CapturedProcessTree::capture(identity).unwrap();
    let mut ownership =
        PtyOwnership::new_for_test(Box::new(child), tree, reserve_fault_reaper().unwrap()).unwrap();
    let payload_dropped = Arc::new(AtomicBool::new(false));
    ownership.inject_emergency_payload_for_test(Box::new(EmergencyPayload(Arc::clone(
        &payload_dropped,
    ))));
    let trace_events = Arc::new(AtomicUsize::new(usize::default()));
    let trace_payload_dropped = Arc::new(AtomicBool::new(false));
    let subscriber = PanicSubscriber {
        events: Arc::clone(&trace_events),
        payload_dropped: Arc::clone(&trace_payload_dropped),
    };

    let _tracing_test = crate::tracing_test_support::serialize_subscriber_tests();
    let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        tracing::subscriber::with_default(subscriber, || {
            let _ownership = ownership;
            panic!("{OUTER_PANIC_DETAIL}");
        });
    }));

    let payload = outcome.unwrap_err();
    assert_eq!(panic_message(payload.as_ref()), Some(OUTER_PANIC_DETAIL));
    assert_eq!(trace_events.load(Ordering::SeqCst), usize::default());
    assert!(!trace_payload_dropped.load(Ordering::SeqCst));
    wait_until_gone(identity);
    assert!(payload_dropped.load(Ordering::SeqCst));
}

fn wait_until_gone(identity: ProcessIdentity) {
    let deadline = Instant::now() + WAIT_TIMEOUT;
    while identity.matches() {
        assert!(Instant::now() < deadline);
        std::thread::sleep(WAIT_INTERVAL);
    }
}

fn panic_message(payload: &(dyn std::any::Any + Send)) -> Option<&str> {
    payload
        .downcast_ref::<&str>()
        .copied()
        .or_else(|| payload.downcast_ref::<String>().map(String::as_str))
}

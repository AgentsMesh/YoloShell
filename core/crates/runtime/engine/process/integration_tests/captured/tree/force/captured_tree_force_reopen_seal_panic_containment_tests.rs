use super::exact_gate_fixture::ExactGateFixture;
use crate::process::pty_session_proof::fail_reopen_seal_recovery_for_test;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use tracing::span::{Attributes, Id, Record};
use tracing::{Event, Metadata, Subscriber};

const TEST_CAPABILITY: &str = "reopen-seal-panic-containment";
const ORIGINAL_DROP_PANIC: &str = "injected reopen-seal transaction payload Drop panic";
const SUBSCRIBER_DROP_PANIC: &str = "injected reopen-seal subscriber payload Drop panic";
const ORIGINAL_PAYLOAD_REPLACED_DETAIL: &str =
    "reopen-seal cleanup replaced the original transaction panic";
const ORIGINAL_PAYLOAD_DROPPED_DETAIL: &str =
    "reopen-seal cleanup dropped the original transaction panic";
const CLEANUP_FAILURES_NOT_ARMED_DETAIL: &str =
    "reopen-seal transaction panic occurred before persistent cleanup failures were armed";
const QUARANTINE_MISSING_DETAIL: &str = "reopen-seal panic cleanup did not reach quarantine";
const TRACE_EVENT_COUNT: usize = 1;
const TEST_SPAN_ID: u64 = 1;

struct OriginalPayload(Arc<AtomicUsize>);

impl Drop for OriginalPayload {
    fn drop(&mut self) {
        self.0.fetch_add(1, Ordering::SeqCst);
        panic!("{ORIGINAL_DROP_PANIC}");
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
        std::panic::panic_any(SubscriberPayload(Arc::clone(&self.payload_drops)));
    }

    fn enter(&self, _span: &Id) {}
    fn exit(&self, _span: &Id) {}
}

struct SubscriberPayload(Arc<AtomicUsize>);

impl Drop for SubscriberPayload {
    fn drop(&mut self) {
        self.0.fetch_add(1, Ordering::SeqCst);
        panic!("{SUBSCRIBER_DROP_PANIC}");
    }
}

#[test]
fn transaction_panic_survives_persistent_cleanup_and_subscriber_panics() {
    let mut fixture = ExactGateFixture::spawn(TEST_CAPABILITY, false);
    let mut frozen = fixture.take_frozen();
    let version = frozen.closure_version();
    let original_drops = Arc::new(AtomicUsize::default());
    let events = Arc::new(AtomicUsize::default());
    let subscriber_drops = Arc::new(AtomicUsize::default());
    let subscriber = PanicSubscriber {
        events: Arc::clone(&events),
        payload_drops: Arc::clone(&subscriber_drops),
    };
    let mut failures = None;
    let _tracing_test = yolo_application_test_fixtures::tracing::serialize_subscriber_tests();

    let outcome = catch_unwind(AssertUnwindSafe(|| {
        tracing::subscriber::with_default(subscriber, || {
            let _ = frozen.prepare_kill_with_destructive_commit(
                version,
                || -> anyhow::Result<()> {
                    failures = Some(fail_reopen_seal_recovery_for_test());
                    std::panic::panic_any(OriginalPayload(Arc::clone(&original_drops)))
                },
                || {},
            );
        });
    }));

    let payload = outcome.expect_err(ORIGINAL_PAYLOAD_REPLACED_DETAIL);
    let original_survived = payload.downcast_ref::<OriginalPayload>().is_some();
    std::mem::forget(payload);
    assert!(original_survived, "{ORIGINAL_PAYLOAD_REPLACED_DETAIL}");
    assert!(failures.is_some(), "{CLEANUP_FAILURES_NOT_ARMED_DETAIL}");
    assert_eq!(
        original_drops.load(Ordering::SeqCst),
        0,
        "{ORIGINAL_PAYLOAD_DROPPED_DETAIL}"
    );
    assert!(
        frozen.reopen_seal_is_quarantined_for_test(),
        "{QUARANTINE_MISSING_DETAIL}"
    );
    assert_eq!(events.load(Ordering::SeqCst), TRACE_EVENT_COUNT);
    assert_eq!(subscriber_drops.load(Ordering::SeqCst), TRACE_EVENT_COUNT);
}

use crate::worker::tests::*;
use tracing::span::{
    Attributes as FinalizationAttributes, Id as FinalizationSpanId, Record as FinalizationRecord,
};
use tracing::{
    Event as FinalizationEvent, Metadata as FinalizationMetadata,
    Subscriber as FinalizationSubscriber,
};

const FINALIZATION_TRACE_EXIT_CODE: i32 = 17;
const FINALIZATION_TRACE_SPAN_ID: u64 = 1;
const FINALIZATION_TRACE_PAYLOAD_PANIC: &str = "injected finalization trace payload Drop panic";

struct FinalizationPanicSubscriber {
    events: Arc<std::sync::atomic::AtomicUsize>,
    payload_dropped: Arc<AtomicBool>,
}

impl FinalizationSubscriber for FinalizationPanicSubscriber {
    fn enabled(&self, _metadata: &FinalizationMetadata<'_>) -> bool {
        true
    }

    fn new_span(&self, _attributes: &FinalizationAttributes<'_>) -> FinalizationSpanId {
        FinalizationSpanId::from_u64(FINALIZATION_TRACE_SPAN_ID)
    }

    fn record(&self, _span: &FinalizationSpanId, _values: &FinalizationRecord<'_>) {}
    fn record_follows_from(&self, _span: &FinalizationSpanId, _follows: &FinalizationSpanId) {}

    fn event(&self, _event: &FinalizationEvent<'_>) {
        self.events.fetch_add(1, Ordering::SeqCst);
        std::panic::panic_any(FinalizationTracePayload(Arc::clone(&self.payload_dropped)));
    }

    fn enter(&self, _span: &FinalizationSpanId) {}
    fn exit(&self, _span: &FinalizationSpanId) {}
}

struct FinalizationTracePayload(Arc<AtomicBool>);

impl Drop for FinalizationTracePayload {
    fn drop(&mut self) {
        self.0.store(true, Ordering::SeqCst);
        panic!("{FINALIZATION_TRACE_PAYLOAD_PANIC}");
    }
}

#[tokio::test]
async fn exit_persistence_retry_survives_subscriber_panic_after_finalization_commit() {
    let fixture = Fixture::new();
    fixture.state.reader_lifecycle.mark_drained();
    fixture.state.control_test_hooks.block_exit_persist();
    let events = Arc::new(std::sync::atomic::AtomicUsize::default());
    let payload_dropped = Arc::new(AtomicBool::default());
    let subscriber = FinalizationPanicSubscriber {
        events: Arc::clone(&events),
        payload_dropped: Arc::clone(&payload_dropped),
    };
    let state = Arc::clone(&fixture.state);
    let finalizer = std::thread::spawn(move || {
        let _tracing_test = crate::tracing_test_support::serialize_subscriber_tests();
        tracing::subscriber::with_default(subscriber, || {
            finalize_child_exit_scheduled(
                &state,
                Some(FINALIZATION_TRACE_EXIT_CODE),
                None,
                FinalizationSchedule {
                    drain_interval: Duration::ZERO,
                    blocked_after: Duration::ZERO,
                },
            );
        });
    });

    wait_for_runtime_status(
        &fixture.state,
        TerminalRuntimeStatus::OutputBlocked,
        EXIT_PERSISTENCE_TIMEOUT,
    )
    .await;
    assert!(lock_recover(&fixture.state.exit).is_none());
    fixture.state.control_test_hooks.allow_exit_persist();
    finalizer.join().unwrap();

    assert!(events.load(Ordering::SeqCst) > 0);
    assert!(!payload_dropped.load(Ordering::SeqCst));
    assert_eq!(fixture.state.status().status, TerminalRuntimeStatus::Exited);
    assert_eq!(
        lock_recover(&fixture.state.exit).as_ref().unwrap().code,
        Some(FINALIZATION_TRACE_EXIT_CODE)
    );
}

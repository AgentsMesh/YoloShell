use std::sync::Arc;
use tracing::span::{Attributes, Id, Record};
use tracing::{Event, Metadata, Subscriber};

const COMPLETION_OUTER_PANIC: &str = "injected completion outer unwind";
const COMPLETION_DISPLAY_PANIC: &str = "injected completion error Display panic";
const COMPLETION_PAYLOAD_DROP_PANIC: &str = "injected completion payload Drop panic";
const COMPLETION_TEST_SPAN_ID: u64 = 1;

#[derive(Debug)]
struct CompletionHostileError {
    error_dropped: Arc<std::sync::atomic::AtomicBool>,
    payload_dropped: Arc<std::sync::atomic::AtomicBool>,
}

impl std::fmt::Display for CompletionHostileError {
    fn fmt(&self, _formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        panic!("{COMPLETION_DISPLAY_PANIC}");
    }
}

impl std::error::Error for CompletionHostileError {}

impl Drop for CompletionHostileError {
    fn drop(&mut self) {
        self.error_dropped
            .store(true, std::sync::atomic::Ordering::SeqCst);
        std::panic::panic_any(CompletionDropPayload(Arc::clone(&self.payload_dropped)));
    }
}

struct CompletionDropPayload(Arc<std::sync::atomic::AtomicBool>);

impl Drop for CompletionDropPayload {
    fn drop(&mut self) {
        self.0.store(true, std::sync::atomic::Ordering::SeqCst);
        panic!("{COMPLETION_PAYLOAD_DROP_PANIC}");
    }
}

struct CompletionPanicSubscriber {
    events: Arc<std::sync::atomic::AtomicUsize>,
    payload_dropped: Arc<std::sync::atomic::AtomicBool>,
}

impl Subscriber for CompletionPanicSubscriber {
    fn enabled(&self, _metadata: &Metadata<'_>) -> bool {
        true
    }

    fn new_span(&self, _attributes: &Attributes<'_>) -> Id {
        Id::from_u64(COMPLETION_TEST_SPAN_ID)
    }

    fn record(&self, _span: &Id, _values: &Record<'_>) {}
    fn record_follows_from(&self, _span: &Id, _follows: &Id) {}

    fn event(&self, _event: &Event<'_>) {
        self.events
            .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        std::panic::panic_any(CompletionDropPayload(Arc::clone(&self.payload_dropped)));
    }

    fn enter(&self, _span: &Id) {}
    fn exit(&self, _span: &Id) {}
}

struct CompletionPublishOnDrop {
    completion: FaultReapCompletion,
    outcome: Option<ReapOutcome>,
}

impl Drop for CompletionPublishOnDrop {
    fn drop(&mut self) {
        self.completion
            .publish(self.outcome.take().expect("completion outcome is present"));
    }
}

#[test]
fn log_target_contains_error_and_trace_panics_during_outer_unwind() {
    let completion = FaultReapCompletion::log();
    let flags = CompletionFlags::new();
    let outcome = hostile_outcome(&flags);

    run_outer_unwind(completion.clone(), outcome, &flags);

    assert!(completion.is_published());
    flags.assert_contained(true);
}

#[test]
fn panicking_target_contains_outcome_and_trace_payloads_during_outer_unwind() {
    let completion = FaultReapCompletion::drop_panicking_for_test();
    let flags = CompletionFlags::new();
    let outcome = hostile_outcome(&flags);
    let trace_events = Arc::clone(&flags.trace_events);
    let trace_payload_dropped = Arc::clone(&flags.trace_payload_dropped);
    FaultReapCompletion::install_panic_trace_hook_for_test(move || {
        trace_events.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        std::panic::panic_any(CompletionDropPayload(trace_payload_dropped));
    });

    run_outer_unwind(completion.clone(), outcome, &flags);

    assert!(completion.is_published());
    assert!(completion.publication_fault_for_test().is_some());
    flags.assert_contained(true);
}

#[test]
fn duplicate_publication_contains_rejected_outcome_drop_during_outer_unwind() {
    let completion = FaultReapCompletion::log();
    completion.publish(ReapOutcome::waited(None));
    let flags = CompletionFlags::new();
    let outcome = hostile_outcome(&flags);

    run_outer_unwind(completion, outcome, &flags);

    flags.assert_contained(false);
}

struct CompletionFlags {
    error_dropped: Arc<std::sync::atomic::AtomicBool>,
    error_payload_dropped: Arc<std::sync::atomic::AtomicBool>,
    trace_events: Arc<std::sync::atomic::AtomicUsize>,
    trace_payload_dropped: Arc<std::sync::atomic::AtomicBool>,
}

impl CompletionFlags {
    fn new() -> Self {
        Self {
            error_dropped: Arc::new(std::sync::atomic::AtomicBool::default()),
            error_payload_dropped: Arc::new(std::sync::atomic::AtomicBool::default()),
            trace_events: Arc::new(std::sync::atomic::AtomicUsize::default()),
            trace_payload_dropped: Arc::new(std::sync::atomic::AtomicBool::default()),
        }
    }

    fn assert_contained(&self, trace_expected: bool) {
        assert!(self.error_dropped.load(std::sync::atomic::Ordering::SeqCst));
        assert!(
            !self
                .error_payload_dropped
                .load(std::sync::atomic::Ordering::SeqCst)
        );
        assert_eq!(
            self.trace_events.load(std::sync::atomic::Ordering::SeqCst) > 0,
            trace_expected
        );
        assert!(
            !self
                .trace_payload_dropped
                .load(std::sync::atomic::Ordering::SeqCst)
        );
    }
}

fn hostile_outcome(flags: &CompletionFlags) -> ReapOutcome {
    let mut outcome = ReapOutcome::lost_unknown();
    outcome.fault = Some(anyhow::Error::new(CompletionHostileError {
        error_dropped: Arc::clone(&flags.error_dropped),
        payload_dropped: Arc::clone(&flags.error_payload_dropped),
    }));
    outcome
}

fn run_outer_unwind(
    completion: FaultReapCompletion,
    outcome: ReapOutcome,
    flags: &CompletionFlags,
) {
    let _tracing_test = crate::tracing_test_support::serialize_subscriber_tests();
    let subscriber = CompletionPanicSubscriber {
        events: Arc::clone(&flags.trace_events),
        payload_dropped: Arc::clone(&flags.trace_payload_dropped),
    };
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        tracing::subscriber::with_default(subscriber, || {
            let _publish = CompletionPublishOnDrop {
                completion,
                outcome: Some(outcome),
            };
            panic!("{COMPLETION_OUTER_PANIC}");
        });
    }));
    assert_eq!(
        completion_panic_message(result.unwrap_err().as_ref()),
        Some(COMPLETION_OUTER_PANIC)
    );
}

fn completion_panic_message(payload: &(dyn std::any::Any + Send)) -> Option<&str> {
    payload
        .downcast_ref::<&str>()
        .copied()
        .or_else(|| payload.downcast_ref::<String>().map(String::as_str))
}

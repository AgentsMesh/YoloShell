use super::*;
use crate::process::{CapturedProcessTree, ProcessIdentity};
use std::process::Command;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::time::Duration;
use tracing::span::{Attributes, Id, Record};
use tracing::{Event, Metadata, Subscriber};

const LOOP_COMMAND: &str = "while :; do :; done";
const SHELL_ARGUMENT: &str = "-c";
const SHELL_PATH: &str = "/bin/sh";
const FAULT_REAPER_TEST_TIMEOUT: Duration = Duration::from_secs(10);
const ROOT_STATUS_MISSING_DETAIL: &str = "fault reaper did not retain the root wait status";
const HANDOFF_OUTER_PANIC: &str = "injected PTY handoff outer unwind";
const HANDOFF_TRACE_PAYLOAD_PANIC: &str = "injected PTY handoff trace payload Drop panic";
const HANDOFF_TEST_SPAN_ID: u64 = 1;

struct HandoffPanicSubscriber {
    events: Arc<AtomicUsize>,
    payload_dropped: Arc<AtomicBool>,
}

impl Subscriber for HandoffPanicSubscriber {
    fn enabled(&self, _metadata: &Metadata<'_>) -> bool {
        true
    }

    fn new_span(&self, _attributes: &Attributes<'_>) -> Id {
        Id::from_u64(HANDOFF_TEST_SPAN_ID)
    }

    fn record(&self, _span: &Id, _values: &Record<'_>) {}
    fn record_follows_from(&self, _span: &Id, _follows: &Id) {}

    fn event(&self, _event: &Event<'_>) {
        self.events.fetch_add(1, Ordering::SeqCst);
        std::panic::panic_any(HandoffTracePayload(Arc::clone(&self.payload_dropped)));
    }

    fn enter(&self, _span: &Id) {}
    fn exit(&self, _span: &Id) {}
}

struct HandoffTracePayload(Arc<AtomicBool>);

impl Drop for HandoffTracePayload {
    fn drop(&mut self) {
        self.0.store(true, Ordering::SeqCst);
        panic!("{HANDOFF_TRACE_PAYLOAD_PANIC}");
    }
}

struct HandoffOnDrop {
    ownership: PtyOwnership,
    completion: Option<FaultReapCompletion>,
}

impl Drop for HandoffOnDrop {
    fn drop(&mut self) {
        let settlement = self
            .ownership
            .settle_after_fault(self.completion.take().unwrap());
        assert!(matches!(settlement, FaultSettlement::Deferred(_)));
    }
}

#[test]
fn owner_fault_hands_complete_custody_to_typed_reaper() {
    assert_fault_handoff(reserve_fault_reaper().unwrap());
}

#[test]
fn closed_primary_receiver_uses_reserved_fallback_without_losing_custody() {
    assert_fault_handoff(
        crate::worker::pty_fault_reaper::FaultReaperReservation::reserve_primary_closed_for_test()
            .unwrap(),
    );
}

#[test]
fn closed_primary_handoff_contains_trace_panic_during_outer_unwind() {
    let reaper =
        crate::worker::pty_fault_reaper::FaultReaperReservation::reserve_primary_closed_for_test()
            .unwrap();
    let child = crate::process::spawn_waitable_test_child(
        Command::new(SHELL_PATH).args([SHELL_ARGUMENT, LOOP_COMMAND]),
    );
    let identity = ProcessIdentity::capture(child.id()).unwrap();
    let tree = CapturedProcessTree::capture(identity).unwrap();
    let ownership = PtyOwnership::new_for_test(Box::new(child), tree, reaper).unwrap();
    let (completion, receiver) = FaultReapCompletion::owner_channel();
    let events = Arc::new(AtomicUsize::default());
    let payload_dropped = Arc::new(AtomicBool::default());
    let subscriber = HandoffPanicSubscriber {
        events: Arc::clone(&events),
        payload_dropped: Arc::clone(&payload_dropped),
    };

    let _tracing_test = crate::tracing_test_support::serialize_subscriber_tests();
    let panic = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        tracing::subscriber::with_default(subscriber, || {
            let _handoff = HandoffOnDrop {
                ownership,
                completion: Some(completion),
            };
            panic!("{HANDOFF_OUTER_PANIC}");
        });
    }));

    assert_eq!(
        panic_message(panic.unwrap_err().as_ref()),
        Some(HANDOFF_OUTER_PANIC)
    );
    assert_eq!(events.load(Ordering::SeqCst), 1);
    assert!(!payload_dropped.load(Ordering::SeqCst));
    let outcome = receiver.receive_timeout(FAULT_REAPER_TEST_TIMEOUT).unwrap();
    assert_eq!(outcome.evidence, ReapEvidence::Waited);
    assert!(outcome.status.is_some());
    assert!(!identity.matches());
}

fn assert_fault_handoff(reaper: crate::worker::pty_fault_reaper::FaultReaperReservation) {
    let child = crate::process::spawn_waitable_test_child(
        Command::new(SHELL_PATH).args([SHELL_ARGUMENT, LOOP_COMMAND]),
    );
    let identity = ProcessIdentity::capture(child.id()).unwrap();
    let tree = CapturedProcessTree::capture(identity).unwrap();
    let output_calls = Arc::new(AtomicUsize::new(usize::default()));
    let output = crate::worker::pty_output_progress::ReaderBackedPtyOutput::observed(Arc::clone(
        &output_calls,
    ));
    let mut ownership =
        PtyOwnership::new_for_test_with_output(Box::new(child), tree, reaper, output).unwrap();
    let (completion, receiver) = FaultReapCompletion::owner_channel();

    let settlement = ownership.settle_after_fault(completion);

    assert!(matches!(settlement, FaultSettlement::Deferred(_)));
    let outcome = receiver.receive_timeout(FAULT_REAPER_TEST_TIMEOUT).unwrap();
    let status = outcome.status.expect(ROOT_STATUS_MISSING_DETAIL);
    assert_eq!(outcome.evidence, ReapEvidence::Waited);
    assert_eq!(status.signal(), Some(libc::SIGHUP));
    assert!(outcome.fault.is_none());
    assert!(output_calls.load(Ordering::Acquire) > usize::default());
    assert!(!identity.matches());
}

fn panic_message(payload: &(dyn std::any::Any + Send)) -> Option<&str> {
    payload
        .downcast_ref::<&str>()
        .copied()
        .or_else(|| payload.downcast_ref::<String>().map(String::as_str))
}

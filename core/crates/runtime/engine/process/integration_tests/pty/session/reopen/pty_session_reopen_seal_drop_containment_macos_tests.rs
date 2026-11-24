use super::*;
use crate::process::pty_session_proof::macos_reopen_seal_io::{TestIoOperation, fail_next};
use crate::process::pty_session_proof::macos_reopen_seal_panic::{
    TestSealOperation, panic_next_with_payload_drop,
};
use portable_pty::{PtySize, native_pty_system};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use tracing::span::{Attributes, Id, Record};
use tracing::{Event, Metadata, Subscriber};

const DROP_TEST_CAPABILITY: &str = "pty-reopen-seal-drop-containment";
const DROP_TEST_OUTER_PANIC: &str = "injected reopen-seal outer unwind";
const DROP_TEST_TRACE_PAYLOAD_PANIC: &str = "injected reopen-seal trace payload Drop panic";
const DROP_TEST_PATH_RETAINED_DETAIL: &str =
    "failed reopen-seal Drop retained exact PTY descriptor custody";
const DROP_TEST_ROWS: u16 = 24;
const DROP_TEST_COLUMNS: u16 = 80;
const DROP_TEST_PIXEL_SIZE: u16 = 0;
const DROP_TEST_SPAN_ID: u64 = 1;

struct DropTestSubscriber {
    events: Arc<AtomicUsize>,
    payload_dropped: Arc<AtomicBool>,
}

impl Subscriber for DropTestSubscriber {
    fn enabled(&self, _metadata: &Metadata<'_>) -> bool {
        true
    }

    fn new_span(&self, _attributes: &Attributes<'_>) -> Id {
        Id::from_u64(DROP_TEST_SPAN_ID)
    }

    fn record(&self, _span: &Id, _values: &Record<'_>) {}
    fn record_follows_from(&self, _span: &Id, _follows: &Id) {}

    fn event(&self, _event: &Event<'_>) {
        self.events.fetch_add(1, Ordering::SeqCst);
        std::panic::panic_any(DropTestPayload(Arc::clone(&self.payload_dropped)));
    }

    fn enter(&self, _span: &Id) {}
    fn exit(&self, _span: &Id) {}
}

struct DropTestPayload(Arc<AtomicBool>);

impl Drop for DropTestPayload {
    fn drop(&mut self) {
        self.0.store(true, Ordering::SeqCst);
        panic!("{DROP_TEST_TRACE_PAYLOAD_PANIC}");
    }
}

#[test]
fn restore_panic_closes_exact_custody_during_outer_unwind() {
    let (proof, path) = sealed_proof_without_external_descriptors();
    let restore_payload_dropped = Arc::new(AtomicBool::default());
    panic_next_with_payload_drop(
        TestSealOperation::Restore,
        Arc::clone(&restore_payload_dropped),
    );
    let trace_payload_dropped = Arc::new(AtomicBool::default());
    let trace_events = Arc::new(AtomicUsize::default());

    drop_proof_during_outer_unwind(
        proof,
        Arc::clone(&trace_events),
        Arc::clone(&trace_payload_dropped),
    );

    assert!(restore_payload_dropped.load(Ordering::SeqCst));
    assert!(trace_payload_dropped.load(Ordering::SeqCst));
    assert_eq!(trace_events.load(Ordering::SeqCst), 1);
    assert_exact_custody_released(&path);
}

#[test]
fn restore_error_closes_exact_custody_during_outer_unwind() {
    let (proof, path) = sealed_proof_without_external_descriptors();
    fail_next(TestIoOperation::ChangeMode);
    let trace_payload_dropped = Arc::new(AtomicBool::default());
    let trace_events = Arc::new(AtomicUsize::default());

    drop_proof_during_outer_unwind(
        proof,
        Arc::clone(&trace_events),
        Arc::clone(&trace_payload_dropped),
    );

    assert!(trace_payload_dropped.load(Ordering::SeqCst));
    assert_eq!(trace_events.load(Ordering::SeqCst), 1);
    assert_exact_custody_released(&path);
}

fn sealed_proof_without_external_descriptors() -> (PtySessionProof, std::path::PathBuf) {
    let pair = native_pty_system()
        .openpty(PtySize {
            rows: DROP_TEST_ROWS,
            cols: DROP_TEST_COLUMNS,
            pixel_width: DROP_TEST_PIXEL_SIZE,
            pixel_height: DROP_TEST_PIXEL_SIZE,
        })
        .unwrap();
    let path = pair.master.tty_name().unwrap();
    let mut proof =
        PtySessionProof::capture_bound(pair.master.as_ref(), DROP_TEST_CAPABILITY.to_owned())
            .unwrap();
    let authority = proof.acquire_reopen_seal().unwrap();
    proof.install_reopen_seal(authority).unwrap();
    drop(pair);
    (proof, path)
}

fn drop_proof_during_outer_unwind(
    proof: PtySessionProof,
    trace_events: Arc<AtomicUsize>,
    trace_payload_dropped: Arc<AtomicBool>,
) {
    let _tracing_test = yolo_application_test_fixtures::tracing::serialize_subscriber_tests();
    let subscriber = DropTestSubscriber {
        events: trace_events,
        payload_dropped: trace_payload_dropped,
    };
    let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        tracing::subscriber::with_default(subscriber, || {
            let _proof = proof;
            panic!("{DROP_TEST_OUTER_PANIC}");
        });
    }));
    assert_eq!(
        drop_test_panic_message(outcome.unwrap_err().as_ref()),
        Some(DROP_TEST_OUTER_PANIC)
    );
}

fn assert_exact_custody_released(path: &std::path::Path) {
    assert!(!path.exists(), "{DROP_TEST_PATH_RETAINED_DETAIL}");
}

fn drop_test_panic_message(payload: &(dyn std::any::Any + Send)) -> Option<&str> {
    payload
        .downcast_ref::<&str>()
        .copied()
        .or_else(|| payload.downcast_ref::<String>().map(String::as_str))
}

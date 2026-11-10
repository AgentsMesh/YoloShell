use crate::host::tests::*;
use super::*;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::task::{Context, Poll};
use tracing::span::{Attributes, Id, Record};
use tracing::{Event, Metadata, Subscriber};

const EXPECTED_CONTAINED_FAILURES: usize = 1;
const RECONCILE_FUTURE_DROP_PANIC: &str = "injected reconcile future Drop panic";
const RECONCILE_OUTER_PANIC: &str = "injected reconcile poll panic";
const RECONCILE_PAYLOAD_DROP_PANIC: &str = "injected reconcile payload Drop panic";
const RECONCILE_SUBSCRIBER_PANIC: &str = "injected reconcile subscriber panic";
const RECONCILE_TEST_SPAN_ID: u64 = 1;

struct ReconcilePanicPayload(Arc<AtomicBool>);

impl Drop for ReconcilePanicPayload {
    fn drop(&mut self) {
        self.0.store(true, Ordering::Release);
        panic!("{RECONCILE_PAYLOAD_DROP_PANIC}");
    }
}

struct PanicPollFuture {
    future_dropped: Arc<AtomicBool>,
    payload_dropped: Arc<AtomicBool>,
}

impl Future for PanicPollFuture {
    type Output = ReconcileResult;

    fn poll(self: Pin<&mut Self>, _context: &mut Context<'_>) -> Poll<Self::Output> {
        std::panic::panic_any(ReconcilePanicPayload(Arc::clone(&self.payload_dropped)))
    }
}

impl Drop for PanicPollFuture {
    fn drop(&mut self) {
        self.future_dropped.store(true, Ordering::Release);
        panic!("{RECONCILE_FUTURE_DROP_PANIC}");
    }
}

struct PendingReconcileFuture {
    drop_observed: Arc<AtomicBool>,
    started: Option<tokio::sync::oneshot::Sender<()>>,
}

impl Future for PendingReconcileFuture {
    type Output = ReconcileResult;

    fn poll(mut self: Pin<&mut Self>, _context: &mut Context<'_>) -> Poll<Self::Output> {
        if let Some(started) = self.started.take() {
            let _ = started.send(());
        }
        Poll::Pending
    }
}

impl Drop for PendingReconcileFuture {
    fn drop(&mut self) {
        self.drop_observed.store(true, Ordering::Release);
        panic!("{RECONCILE_FUTURE_DROP_PANIC}");
    }
}

struct ReconcilePanicSubscriber;

impl Subscriber for ReconcilePanicSubscriber {
    fn enabled(&self, _metadata: &Metadata<'_>) -> bool {
        true
    }

    fn new_span(&self, _attributes: &Attributes<'_>) -> Id {
        Id::from_u64(RECONCILE_TEST_SPAN_ID)
    }

    fn record(&self, _span: &Id, _values: &Record<'_>) {}
    fn record_follows_from(&self, _span: &Id, _follows: &Id) {}

    fn event(&self, _event: &Event<'_>) {
        panic!("{RECONCILE_SUBSCRIBER_PANIC}");
    }

    fn enter(&self, _span: &Id) {}
    fn exit(&self, _span: &Id) {}
}

#[tokio::test]
async fn reconcile_contains_poll_payload_and_future_drop_panics() {
    let future_dropped = Arc::new(AtomicBool::default());
    let payload_dropped = Arc::new(AtomicBool::default());
    let result = contain_session_reconciliation(
        Uuid::new_v4(),
        PanicPollFuture {
            future_dropped: Arc::clone(&future_dropped),
            payload_dropped: Arc::clone(&payload_dropped),
        },
    )
    .await;

    assert_eq!(result.unreachable, EXPECTED_CONTAINED_FAILURES);
    assert!(future_dropped.load(Ordering::Acquire));
    assert!(payload_dropped.load(Ordering::Acquire));
}

#[tokio::test]
async fn cancelled_reconcile_contains_pending_future_drop_panic() {
    let drop_observed = Arc::new(AtomicBool::default());
    let (started, startup) = tokio::sync::oneshot::channel();
    let task = tokio::spawn(contain_session_reconciliation(
        Uuid::new_v4(),
        PendingReconcileFuture {
            drop_observed: Arc::clone(&drop_observed),
            started: Some(started),
        },
    ));

    startup.await.unwrap();
    task.abort();
    assert!(task.await.unwrap_err().is_cancelled());
    assert!(drop_observed.load(Ordering::Acquire));
}

#[tokio::test(flavor = "current_thread")]
async fn reconcile_contains_tracing_subscriber_panic() {
    let _tracing_test = crate::tracing_test_support::serialize_subscriber_tests();
    let _subscriber = tracing::subscriber::set_default(ReconcilePanicSubscriber);
    let result = contain_session_reconciliation(Uuid::new_v4(), async {
        panic!("{RECONCILE_OUTER_PANIC}");
    })
    .await;

    assert_eq!(result.unreachable, EXPECTED_CONTAINED_FAILURES);
}

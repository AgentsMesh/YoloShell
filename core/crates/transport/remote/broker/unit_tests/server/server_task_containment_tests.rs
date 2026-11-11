use super::*;
use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};

const FUTURE_DROP_PANIC: &str = "injected broker task future Drop panic";
const OUTER_PANIC: &str = "injected broker task outer panic";
const PAYLOAD_DROP_PANIC: &str = "injected broker task payload Drop panic";

struct PanicOnDropPayload;

impl Drop for PanicOnDropPayload {
    fn drop(&mut self) {
        panic!("{PAYLOAD_DROP_PANIC}");
    }
}

struct PendingDropFuture {
    started: Option<tokio::sync::oneshot::Sender<()>>,
}

impl Future for PendingDropFuture {
    type Output = ();

    fn poll(mut self: Pin<&mut Self>, _context: &mut Context<'_>) -> Poll<Self::Output> {
        if let Some(started) = self.started.take() {
            let _ = started.send(());
        }
        Poll::Pending
    }
}

impl Drop for PendingDropFuture {
    fn drop(&mut self) {
        panic!("{FUTURE_DROP_PANIC}");
    }
}

struct PanicDropValue;

impl Drop for PanicDropValue {
    fn drop(&mut self) {
        std::panic::panic_any(PanicOnDropPayload);
    }
}

#[tokio::test(flavor = "current_thread")]
async fn broker_task_panic_payload_drop_is_contained() {
    let task = tokio::spawn(contain_task(async {
        std::panic::panic_any(PanicOnDropPayload);
    }));

    task.await.unwrap();
}

#[tokio::test(flavor = "current_thread")]
async fn broker_task_cancellation_contains_future_drop_panic() {
    let (started, observed) = tokio::sync::oneshot::channel();
    let task = tokio::spawn(contain_task(PendingDropFuture {
        started: Some(started),
    }));
    observed.await.unwrap();

    task.abort();
    let cancellation = task.await.unwrap_err();
    assert!(cancellation.is_cancelled());
    assert!(!cancellation.is_panic());
}

#[test]
fn broker_task_guard_contains_secondary_panic_during_outer_unwind() {
    let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let _guard = TaskFutureGuard::new(PanicDropValue);
        panic!("{OUTER_PANIC}");
    }));

    let payload = outcome.unwrap_err();
    let detail = payload
        .downcast_ref::<&str>()
        .copied()
        .or_else(|| payload.downcast_ref::<String>().map(String::as_str));
    assert_eq!(detail, Some(OUTER_PANIC));
}

use super::*;
use std::fmt;
use std::future::Future;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::pin::Pin;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::task::{Context, Poll};

const ERROR_DISPLAY_PANIC: &str = "injected secondary-custody error Display panic";
const ERROR_DROP_PANIC: &str = "injected secondary-custody error Drop panic";
const FUTURE_DROP_PANIC: &str = "injected secondary-custody future Drop panic";
const OUTER_UNWIND: &str = "injected outer secondary-custody unwind";
const PAYLOAD_DROP_PANIC: &str = "injected secondary-custody payload Drop panic";

#[derive(Debug)]
struct PanicError {
    displayed: Arc<AtomicBool>,
    dropped: Arc<AtomicBool>,
    payload_dropped: Arc<AtomicBool>,
}

impl fmt::Display for PanicError {
    fn fmt(&self, _formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.displayed.store(true, Ordering::Release);
        panic!("{ERROR_DISPLAY_PANIC}");
    }
}

impl std::error::Error for PanicError {}

impl Drop for PanicError {
    fn drop(&mut self) {
        self.dropped.store(true, Ordering::Release);
        std::panic::panic_any(PanicPayload(Arc::clone(&self.payload_dropped)));
    }
}

struct PanicPayload(Arc<AtomicBool>);

impl Drop for PanicPayload {
    fn drop(&mut self) {
        self.0.store(true, Ordering::Release);
        panic!("{PAYLOAD_DROP_PANIC}");
    }
}

struct ErrorDuringUnwind(Option<anyhow::Error>);

impl Drop for ErrorDuringUnwind {
    fn drop(&mut self) {
        let error = self.0.take().expect(ERROR_DROP_PANIC);
        drop(super::containment::error_detail(error));
    }
}

#[test]
fn settlement_error_display_and_drop_panics_are_contained_during_outer_unwind() {
    let displayed = Arc::new(AtomicBool::default());
    let dropped = Arc::new(AtomicBool::default());
    let payload_dropped = Arc::new(AtomicBool::default());
    let error = PanicError {
        displayed: Arc::clone(&displayed),
        dropped: Arc::clone(&dropped),
        payload_dropped: Arc::clone(&payload_dropped),
    };
    let outcome = catch_unwind(AssertUnwindSafe(|| {
        let _error = ErrorDuringUnwind(Some(anyhow::Error::new(error)));
        panic!("{OUTER_UNWIND}");
    }));

    assert_eq!(
        panic_message(outcome.unwrap_err().as_ref()),
        Some(OUTER_UNWIND)
    );
    assert!(displayed.load(Ordering::Acquire));
    assert!(dropped.load(Ordering::Acquire));
    assert!(payload_dropped.load(Ordering::Acquire));
}

struct PanicFuture {
    _fail_closed: super::exchange_guard::PostPreparedFailStop,
    future_dropped: Arc<AtomicBool>,
    payload_dropped: Arc<AtomicBool>,
}

impl Future for PanicFuture {
    type Output = ();

    fn poll(self: Pin<&mut Self>, _context: &mut Context<'_>) -> Poll<Self::Output> {
        std::panic::panic_any(PanicPayload(Arc::clone(&self.payload_dropped)));
    }
}

impl Drop for PanicFuture {
    fn drop(&mut self) {
        self.future_dropped.store(true, Ordering::Release);
        panic!("{FUTURE_DROP_PANIC}");
    }
}

#[tokio::test]
async fn handshake_future_and_payload_drop_panics_still_fail_closed() {
    let directory = tempfile::tempdir().unwrap();
    let host = Host::open(crate::host::HostConfig::new(directory.path())).unwrap();
    let mut fail_closed =
        super::exchange_guard::PostPreparedFailStop::new(host.request_drain.clone());
    fail_closed.arm();
    let future_dropped = Arc::new(AtomicBool::default());
    let payload_dropped = Arc::new(AtomicBool::default());
    let outcome = super::containment::catch_future_detail(PanicFuture {
        _fail_closed: fail_closed,
        future_dropped: Arc::clone(&future_dropped),
        payload_dropped: Arc::clone(&payload_dropped),
    })
    .await;

    assert!(outcome.is_err());
    assert!(future_dropped.load(Ordering::Acquire));
    assert!(payload_dropped.load(Ordering::Acquire));
    assert!(host.request_drain.fail_stop_detail().is_some());
}

fn panic_message(payload: &(dyn std::any::Any + Send)) -> Option<&str> {
    payload
        .downcast_ref::<&str>()
        .copied()
        .or_else(|| payload.downcast_ref::<String>().map(String::as_str))
}

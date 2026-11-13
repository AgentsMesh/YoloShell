use crate::host::tests::*;
use super::*;
use std::fmt;
use std::pin::Pin;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::task::{Context, Poll};

const WATCHDOG_DEBUG_PANIC: &str = "injected watchdog result Debug panic";
const WATCHDOG_DROP_PANIC: &str = "injected watchdog result Drop panic";
const WATCHDOG_EXPECTED_ATTEMPTS: usize = 3;
const WATCHDOG_EXPECTED_PAYLOAD_DROPS: usize = 2;
const WATCHDOG_FACTORY_PANIC_ATTEMPT: usize = 0;
const WATCHDOG_FUTURE_PANIC_ATTEMPT: usize = 1;
const WATCHDOG_PENDING_DROP_PANIC: &str = "injected pending watchdog future Drop panic";
const WATCHDOG_RESULT_DETAIL: &str = "watchdog-test-result";

struct WatchdogPanicPayload(Arc<AtomicUsize>);

impl Drop for WatchdogPanicPayload {
    fn drop(&mut self) {
        self.0.fetch_add(1, Ordering::AcqRel);
        panic!("{WATCHDOG_DROP_PANIC}");
    }
}

enum WatchdogOutcomeMode {
    DebugPanic,
    DropPanic,
    Complete,
}

struct WatchdogOutcome {
    drop_observed: Arc<AtomicBool>,
    mode: WatchdogOutcomeMode,
}

impl fmt::Debug for WatchdogOutcome {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        if matches!(self.mode, WatchdogOutcomeMode::DebugPanic) {
            panic!("{WATCHDOG_DEBUG_PANIC}");
        }
        formatter.write_str(WATCHDOG_RESULT_DETAIL)
    }
}

impl Drop for WatchdogOutcome {
    fn drop(&mut self) {
        if matches!(self.mode, WatchdogOutcomeMode::DropPanic) {
            self.drop_observed.store(true, Ordering::Release);
            panic!("{WATCHDOG_DROP_PANIC}");
        }
    }
}

struct PendingDropFuture {
    drop_observed: Arc<AtomicBool>,
    started: Option<oneshot::Sender<()>>,
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
        self.drop_observed.store(true, Ordering::Release);
        panic!("{WATCHDOG_PENDING_DROP_PANIC}");
    }
}

#[tokio::test]
async fn watchdog_contains_factory_and_future_payload_drop_panics() {
    let attempts = Arc::new(AtomicUsize::default());
    let payload_drops = Arc::new(AtomicUsize::default());
    let (completed, completion) = oneshot::channel();
    let completed = Arc::new(Mutex::new(Some(completed)));
    let (shutdown, receiver) = oneshot::channel();
    let attempt_state = Arc::clone(&attempts);
    let drop_state = Arc::clone(&payload_drops);
    let task = tokio::spawn(run_watchdog(
        test_watchdog_schedule(),
        receiver,
        move || {
            let attempt = attempt_state.fetch_add(1, Ordering::AcqRel);
            if attempt == WATCHDOG_FACTORY_PANIC_ATTEMPT {
                std::panic::panic_any(WatchdogPanicPayload(Arc::clone(&drop_state)));
            }
            let drop_state = Arc::clone(&drop_state);
            let completed = Arc::clone(&completed);
            async move {
                if attempt == WATCHDOG_FUTURE_PANIC_ATTEMPT {
                    std::panic::panic_any(WatchdogPanicPayload(drop_state));
                }
                if let Some(completed) = completed.lock().unwrap().take() {
                    let _ = completed.send(());
                }
            }
        },
    ));

    tokio::time::timeout(TEST_WATCHDOG_TIMEOUT, completion)
        .await
        .unwrap()
        .unwrap();
    shutdown.send(()).unwrap();
    task.await.unwrap();
    assert!(attempts.load(Ordering::Acquire) >= WATCHDOG_EXPECTED_ATTEMPTS);
    assert_eq!(
        payload_drops.load(Ordering::Acquire),
        WATCHDOG_EXPECTED_PAYLOAD_DROPS
    );
}

#[tokio::test]
async fn watchdog_contains_result_debug_and_drop_panics() {
    let attempts = Arc::new(AtomicUsize::default());
    let drop_observed = Arc::new(AtomicBool::default());
    let (completed, completion) = oneshot::channel();
    let completed = Arc::new(Mutex::new(Some(completed)));
    let (shutdown, receiver) = oneshot::channel();
    let attempt_state = Arc::clone(&attempts);
    let drop_state = Arc::clone(&drop_observed);
    let task = tokio::spawn(run_watchdog(
        test_watchdog_schedule(),
        receiver,
        move || {
            let attempt = attempt_state.fetch_add(1, Ordering::AcqRel);
            let mode = match attempt {
                WATCHDOG_FACTORY_PANIC_ATTEMPT => WatchdogOutcomeMode::DebugPanic,
                WATCHDOG_FUTURE_PANIC_ATTEMPT => WatchdogOutcomeMode::DropPanic,
                _ => WatchdogOutcomeMode::Complete,
            };
            let completed = Arc::clone(&completed);
            let drop_observed = Arc::clone(&drop_state);
            async move {
                if matches!(mode, WatchdogOutcomeMode::Complete)
                    && let Some(completed) = completed.lock().unwrap().take()
                {
                    let _ = completed.send(());
                }
                WatchdogOutcome {
                    drop_observed,
                    mode,
                }
            }
        },
    ));

    tokio::time::timeout(TEST_WATCHDOG_TIMEOUT, completion)
        .await
        .unwrap()
        .unwrap();
    shutdown.send(()).unwrap();
    task.await.unwrap();
    assert!(attempts.load(Ordering::Acquire) >= WATCHDOG_EXPECTED_ATTEMPTS);
    assert!(drop_observed.load(Ordering::Acquire));
}

#[tokio::test]
async fn watchdog_shutdown_contains_pending_future_drop_panic() {
    let drop_observed = Arc::new(AtomicBool::default());
    let (started, startup) = oneshot::channel();
    let started = Arc::new(Mutex::new(Some(started)));
    let (shutdown, receiver) = oneshot::channel();
    let drop_state = Arc::clone(&drop_observed);
    let task = tokio::spawn(run_watchdog(
        test_watchdog_schedule(),
        receiver,
        move || PendingDropFuture {
            drop_observed: Arc::clone(&drop_state),
            started: started.lock().unwrap().take(),
        },
    ));

    tokio::time::timeout(TEST_WATCHDOG_TIMEOUT, startup)
        .await
        .unwrap()
        .unwrap();
    shutdown.send(()).unwrap();
    task.await.unwrap();
    assert!(drop_observed.load(Ordering::Acquire));
}

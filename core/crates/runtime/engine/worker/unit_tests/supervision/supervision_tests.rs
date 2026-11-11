use super::*;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::time::Duration;
use tokio::sync::oneshot;

const TEST_RESTART_DELAY: Duration = Duration::from_millis(1);
const TEST_MAX_RESTART_DELAY: Duration = Duration::from_millis(2);
const FIRST_RESTART_ATTEMPT: u32 = 1;
const MONITOR_ERROR_ATTEMPT: usize = 0;
const MONITOR_PANIC_ATTEMPT: usize = 1;
const EXPECTED_MONITOR_ATTEMPTS: usize = 3;
const PAYLOAD_DROP_PANIC_DETAIL: &str = "injected supervision payload Drop panic";
const READER_PANIC_DETAIL: &str = "injected reader panic";
const WAIT_FAILURE_DETAIL: &str = "wait failed";
const ASYNC_PANIC_DETAIL: &str = "injected async panic";
const MONITOR_ERROR_DETAIL: &str = "injected monitor error";
const MONITOR_PANIC_DETAIL: &str = "injected monitor panic";

struct PanicOnDropPayload;

impl Drop for PanicOnDropPayload {
    fn drop(&mut self) {
        panic!("{PAYLOAD_DROP_PANIC_DETAIL}");
    }
}

#[test]
fn thread_panics_and_errors_are_reported_without_unwinding() {
    let panic =
        catch_thread::<(), _>(PTY_READER_TASK, || panic!("{READER_PANIC_DETAIL}")).unwrap_err();
    assert_eq!(panic.task, PTY_READER_TASK);
    assert_eq!(panic.detail, READER_PANIC_DETAIL);
    let error = catch_thread::<(), _>(CHILD_WAITER_TASK, || anyhow::bail!("{WAIT_FAILURE_DETAIL}"))
        .unwrap_err();
    assert_eq!(error.detail, WAIT_FAILURE_DETAIL);
}

#[test]
fn panic_payload_drop_cannot_escape_the_supervision_boundary() {
    let detail = panic_detail(Box::new(PanicOnDropPayload));

    assert_eq!(detail, TASK_PANIC_DETAIL);
}

#[test]
fn secondary_panic_payload_drop_cannot_escape_containment() {
    let outcome = contain::<()>(|| std::panic::panic_any(PanicOnDropPayload));

    assert!(outcome.is_none());
}

#[tokio::test]
async fn direct_joinset_task_reports_panics() {
    for task in [AGENT_MONITOR_TASK, CONNECTION_TASK] {
        let mut tasks = JoinSet::new();
        spawn_task(&mut tasks, task, async move {
            panic!("{ASYNC_PANIC_DETAIL}");
        });
        let failure = tasks.join_next().await.unwrap().unwrap().unwrap();
        assert_eq!(failure.task, task);
    }
}

#[tokio::test]
async fn aborting_joinset_drops_the_owned_future() {
    struct Dropped(Option<oneshot::Sender<()>>);
    impl Drop for Dropped {
        fn drop(&mut self) {
            let _ = self.0.take().unwrap().send(());
        }
    }
    let (started_tx, started_rx) = oneshot::channel();
    let (dropped_tx, dropped_rx) = oneshot::channel();
    let mut tasks = JoinSet::new();
    spawn_task(&mut tasks, CONNECTION_TASK, async move {
        let _guard = Dropped(Some(dropped_tx));
        let _ = started_tx.send(());
        std::future::pending::<()>().await;
        Ok(())
    });
    started_rx.await.unwrap();
    tasks.abort_all();
    while tasks.join_next().await.is_some() {}
    dropped_rx.await.unwrap();
}

#[tokio::test]
async fn restart_loop_rebuilds_after_error_and_panic_with_capped_backoff() {
    let attempts = Arc::new(AtomicUsize::new(usize::default()));
    let stopped = Arc::new(AtomicBool::new(false));
    let operation_attempts = Arc::clone(&attempts);
    let operation_stopped = Arc::clone(&stopped);
    restart_task(
        AGENT_MONITOR_TASK,
        RestartPolicy {
            initial_delay: TEST_RESTART_DELAY,
            max_delay: TEST_MAX_RESTART_DELAY,
        },
        move || {
            let attempt = operation_attempts.fetch_add(1, Ordering::AcqRel);
            let stopped = Arc::clone(&operation_stopped);
            async move {
                match attempt {
                    MONITOR_ERROR_ATTEMPT => anyhow::bail!("{MONITOR_ERROR_DETAIL}"),
                    MONITOR_PANIC_ATTEMPT => panic!("{MONITOR_PANIC_DETAIL}"),
                    _ => stopped.store(true, Ordering::Release),
                }
                Ok(())
            }
        },
        || stopped.load(Ordering::Acquire),
    )
    .await;
    assert_eq!(attempts.load(Ordering::Acquire), EXPECTED_MONITOR_ATTEMPTS);
    let policy = RestartPolicy {
        initial_delay: TEST_RESTART_DELAY,
        max_delay: TEST_MAX_RESTART_DELAY,
    };
    assert_eq!(policy.delay(FIRST_RESTART_ATTEMPT), TEST_RESTART_DELAY);
    assert_eq!(policy.delay(u32::MAX), TEST_MAX_RESTART_DELAY);
}

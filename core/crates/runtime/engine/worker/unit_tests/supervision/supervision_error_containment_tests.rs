use super::*;
use std::fmt;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::time::Duration;

const ERROR_DISPLAY_PANIC: &str = "injected supervised error Display panic";
const ERROR_DROP_PANIC: &str = "injected supervised error Drop panic";
const DROP_ERROR_DETAIL: &str = "supervised error with panicking Drop";
const EXPECTED_RESTART_ATTEMPTS: usize = 3;
const DISPLAY_ERROR_ATTEMPT: usize = 0;
const DROP_ERROR_ATTEMPT: usize = 1;
const TEST_RESTART_DELAY: Duration = Duration::ZERO;

#[derive(Debug)]
struct PanicDisplayError;

impl fmt::Display for PanicDisplayError {
    fn fmt(&self, _formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        panic!("{ERROR_DISPLAY_PANIC}");
    }
}

impl std::error::Error for PanicDisplayError {}

#[derive(Debug)]
struct PanicDropError(Arc<AtomicBool>);

impl fmt::Display for PanicDropError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(DROP_ERROR_DETAIL)
    }
}

impl std::error::Error for PanicDropError {}

impl Drop for PanicDropError {
    fn drop(&mut self) {
        self.0.store(true, Ordering::Release);
        panic!("{ERROR_DROP_PANIC}");
    }
}

#[test]
fn synchronous_error_display_panic_becomes_a_task_failure() {
    let failure = catch_thread::<(), _>(PTY_READER_TASK, || {
        Err(anyhow::Error::new(PanicDisplayError))
    })
    .unwrap_err();

    assert_eq!(failure.task, PTY_READER_TASK);
    assert_eq!(failure.detail, TASK_ERROR_DETAIL);
}

#[tokio::test]
async fn joinset_error_drop_panic_becomes_a_task_failure() {
    let dropped = Arc::new(AtomicBool::new(false));
    let error_dropped = Arc::clone(&dropped);
    let mut tasks = JoinSet::new();
    spawn_task(&mut tasks, CONNECTION_TASK, async move {
        Err(anyhow::Error::new(PanicDropError(error_dropped)))
    });

    let failure = tasks.join_next().await.unwrap().unwrap().unwrap();
    assert_eq!(failure.detail, DROP_ERROR_DETAIL);
    assert!(dropped.load(Ordering::Acquire));
}

#[tokio::test]
async fn restart_loop_survives_error_display_and_drop_panics() {
    let attempts = Arc::new(AtomicUsize::new(usize::default()));
    let stopped = Arc::new(AtomicBool::new(false));
    let dropped = Arc::new(AtomicBool::new(false));
    let operation_attempts = Arc::clone(&attempts);
    let operation_stopped = Arc::clone(&stopped);
    let operation_dropped = Arc::clone(&dropped);

    restart_task(
        AGENT_MONITOR_TASK,
        RestartPolicy {
            initial_delay: TEST_RESTART_DELAY,
            max_delay: TEST_RESTART_DELAY,
        },
        move || {
            let attempt = operation_attempts.fetch_add(1, Ordering::AcqRel);
            let stopped = Arc::clone(&operation_stopped);
            let dropped = Arc::clone(&operation_dropped);
            async move {
                match attempt {
                    DISPLAY_ERROR_ATTEMPT => Err(anyhow::Error::new(PanicDisplayError)),
                    DROP_ERROR_ATTEMPT => Err(anyhow::Error::new(PanicDropError(dropped))),
                    _ => {
                        stopped.store(true, Ordering::Release);
                        Ok(())
                    }
                }
            }
        },
        || stopped.load(Ordering::Acquire),
    )
    .await;

    assert_eq!(attempts.load(Ordering::Acquire), EXPECTED_RESTART_ATTEMPTS);
    assert!(dropped.load(Ordering::Acquire));
}

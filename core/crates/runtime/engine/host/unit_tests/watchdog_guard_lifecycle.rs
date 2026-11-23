use crate::host::tests::*;
use super::*;

const WATCHDOG_DROP_NOT_OBSERVED: &str = "watchdog Drop did not signal shutdown";
const WATCHDOG_DETACHED_TASK_FINISHED_EARLY: &str =
    "detached watchdog task finished before release";
const WATCHDOG_DETACHED_TASK_NOT_FINISHED: &str = "detached watchdog task did not finish";

#[tokio::test]
async fn watchdog_guard_drop_signals_detached_task_to_finish() {
    let (shutdown, receiver) = tokio::sync::oneshot::channel();
    let (observed, observation) = tokio::sync::oneshot::channel();
    let (release, released) = tokio::sync::oneshot::channel();
    let (finished, completion) = tokio::sync::oneshot::channel();
    let task = tokio::spawn(async move {
        let _ = receiver.await;
        let _ = observed.send(());
        let _ = released.await;
        let _ = finished.send(());
    });
    let guard = WatchdogGuard {
        shutdown: Some(shutdown),
        task,
    };

    drop(guard);

    tokio::time::timeout(HOST_BLOCKING_TEST_TIMEOUT, observation)
        .await
        .expect(WATCHDOG_DROP_NOT_OBSERVED)
        .expect(WATCHDOG_DROP_NOT_OBSERVED);
    assert!(
        !completion.is_terminated(),
        "{WATCHDOG_DETACHED_TASK_FINISHED_EARLY}"
    );
    release.send(()).unwrap();
    tokio::time::timeout(HOST_BLOCKING_TEST_TIMEOUT, completion)
        .await
        .expect(WATCHDOG_DETACHED_TASK_NOT_FINISHED)
        .expect(WATCHDOG_DETACHED_TASK_NOT_FINISHED);
}

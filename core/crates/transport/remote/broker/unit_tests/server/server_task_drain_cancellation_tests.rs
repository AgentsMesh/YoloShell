use super::*;
use std::future::{Future, pending};
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::time::Duration;
use tokio::sync::mpsc;

const CURRENT_TASK: &str = "current-task";
const DRAIN_DEADLINE: Duration = Duration::from_secs(30);
const DROP_EVENT_TIMEOUT: Duration = Duration::from_secs(2);
const QUEUED_TASK_A: &str = "queued-task-a";
const QUEUED_TASK_B: &str = "queued-task-b";
const TASK_LABELS: [&str; 3] = [QUEUED_TASK_A, QUEUED_TASK_B, CURRENT_TASK];
const TEST_TASK_GROUP: &str = "cancellation-test";
const TEST_MUTEX_POISON: &str = "poison broker task mutex";
const UNEXPECTED_DRAIN_SETTLEMENT: &str = "pending task drain settled before cancellation";
const BLOCKING_TASK_START_TIMEOUT: Duration = Duration::from_secs(2);

struct DropNotice {
    label: &'static str,
    events: mpsc::UnboundedSender<&'static str>,
}

impl Drop for DropNotice {
    fn drop(&mut self) {
        let _ = self.events.send(self.label);
    }
}

#[tokio::test(flavor = "current_thread")]
async fn canceling_bounded_drain_aborts_current_and_queued_tasks() {
    let (tasks, events) = pending_tasks();
    let deadline = tokio::time::Instant::now() + DRAIN_DEADLINE;
    let mut drain = Box::pin(tasks.finish_before(deadline, TEST_TASK_GROUP));
    poll_pending(&mut drain).await;
    drop(drain);
    expect_all_dropped(events).await;
}

#[tokio::test(flavor = "current_thread")]
async fn canceling_natural_drain_aborts_current_and_queued_tasks() {
    let (tasks, events) = pending_tasks();
    let mut shutdown = Box::pin(pending::<()>());
    let mut drain = Box::pin(tasks.finish_naturally_or_external(shutdown.as_mut()));
    poll_pending(&mut drain).await;
    drop(drain);
    expect_all_dropped(events).await;
}

#[tokio::test(flavor = "current_thread")]
async fn canceling_rotation_drain_aborts_both_runtime_and_hub_groups() {
    let (runtime, runtime_events) = pending_tasks();
    let (hub, hub_events) = pending_tasks();
    let mut shutdown = Box::pin(pending::<()>());
    let mut drain = Box::pin(runtime.finish_with_hub_or_external(&hub, shutdown.as_mut()));
    poll_pending(&mut drain).await;
    drop(drain);
    expect_all_dropped(runtime_events).await;
    expect_all_dropped(hub_events).await;
}

#[tokio::test(flavor = "current_thread")]
async fn dropping_poisoned_owner_aborts_tasks_that_were_never_taken() {
    let (tasks, events) = pending_tasks();
    let poison = catch_unwind(AssertUnwindSafe(|| {
        let _handles = tasks.handles.lock().unwrap();
        panic!("{TEST_MUTEX_POISON}");
    }));
    assert!(poison.is_err());
    drop(tasks);
    expect_all_dropped(events).await;
}

#[tokio::test(start_paused = true)]
async fn abort_settlement_returns_at_deadline_for_an_unabortable_blocking_task() {
    let (started, observed) = std::sync::mpsc::channel();
    let (release, released) = std::sync::mpsc::channel();
    let handle = tokio::task::spawn_blocking(move || {
        started.send(()).unwrap();
        released.recv().unwrap();
    });
    observed.recv_timeout(BLOCKING_TASK_START_TIMEOUT).unwrap();
    let mut tasks = TaskDrain::new(vec![handle]);
    let started_at = tokio::time::Instant::now();
    let mut settlement = Box::pin(abort_and_settle(&mut tasks));

    poll_pending(&mut settlement).await;
    tokio::time::advance(TASK_ABORT_SETTLE_TIMEOUT).await;
    assert!(!settlement.await);

    assert_eq!(
        tokio::time::Instant::now() - started_at,
        TASK_ABORT_SETTLE_TIMEOUT
    );
    release.send(()).unwrap();
    tasks.current().unwrap().await.unwrap();
    tasks.complete_current();
}

fn pending_tasks() -> (BrokerTasks, mpsc::UnboundedReceiver<&'static str>) {
    let tasks = BrokerTasks::default();
    let (events, receiver) = mpsc::unbounded_channel();
    for label in TASK_LABELS {
        let notice = DropNotice {
            label,
            events: events.clone(),
        };
        tasks.spawn(async move {
            let _notice = notice;
            pending::<()>().await;
        });
    }
    drop(events);
    (tasks, receiver)
}

async fn poll_pending<F, T>(future: &mut std::pin::Pin<Box<F>>)
where
    F: Future<Output = T>,
{
    tokio::select! {
        biased;
        _ = future => panic!("{UNEXPECTED_DRAIN_SETTLEMENT}"),
        _ = tokio::task::yield_now() => {}
    }
}

async fn expect_all_dropped(mut events: mpsc::UnboundedReceiver<&'static str>) {
    let mut actual = Vec::with_capacity(TASK_LABELS.len());
    for _ in TASK_LABELS {
        actual.push(
            tokio::time::timeout(DROP_EVENT_TIMEOUT, events.recv())
                .await
                .unwrap()
                .unwrap(),
        );
    }
    actual.sort_unstable();
    let mut expected = TASK_LABELS.to_vec();
    expected.sort_unstable();
    assert_eq!(actual, expected);
}

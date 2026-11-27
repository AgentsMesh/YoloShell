use anyhow::Result;
use futures::FutureExt;
use std::any::Any;
use std::future::Future;
use std::panic::{AssertUnwindSafe, catch_unwind};
use tokio::task::JoinSet;

const TASK_ERROR_DETAIL: &str = "worker task failed without a reportable error";

#[cfg(test)]
pub(super) use super::constants::INITIAL_INPUT_TASK;
pub(super) use super::constants::{
    ACCEPT_TASK, AGENT_MONITOR_TASK, CHILD_WAITER_TASK, CODEX_AUTHORITY_PROJECTION_TASK,
    CONNECTION_TASK, PTY_CUSTODY_TASK, PTY_READER_TASK, SUPERVISION_RESTART_INITIAL_DELAY,
    SUPERVISION_RESTART_MAX_DELAY, SUPERVISION_RESTART_MULTIPLIER, TASK_PANIC_DETAIL,
    TASK_STOPPED_UNEXPECTEDLY, TASK_SUPERVISOR,
};

#[derive(Debug)]
pub(super) struct TaskFailure {
    pub task: &'static str,
    pub detail: String,
}

impl TaskFailure {
    pub(super) fn message(task: &'static str, detail: impl Into<String>) -> Self {
        Self {
            task,
            detail: detail.into(),
        }
    }
}

#[derive(Clone, Copy)]
pub(super) struct RestartPolicy {
    pub initial_delay: std::time::Duration,
    pub max_delay: std::time::Duration,
}

impl Default for RestartPolicy {
    fn default() -> Self {
        Self {
            initial_delay: SUPERVISION_RESTART_INITIAL_DELAY,
            max_delay: SUPERVISION_RESTART_MAX_DELAY,
        }
    }
}

impl RestartPolicy {
    pub(super) fn delay(self, failure_count: u32) -> std::time::Duration {
        let mut delay = self.initial_delay;
        for _ in 1..failure_count {
            delay = delay
                .saturating_mul(SUPERVISION_RESTART_MULTIPLIER)
                .min(self.max_delay);
            if delay >= self.max_delay {
                return self.max_delay;
            }
        }
        delay
    }
}

pub(super) fn catch_thread<T, F>(
    task: &'static str,
    operation: F,
) -> std::result::Result<T, TaskFailure>
where
    F: FnOnce() -> Result<T>,
{
    match catch_unwind(AssertUnwindSafe(operation)) {
        Ok(Ok(value)) => Ok(value),
        Ok(Err(error)) => Err(TaskFailure::message(task, error_detail(error))),
        Err(payload) => Err(TaskFailure::message(task, panic_detail(payload))),
    }
}

pub(super) fn spawn_task<F>(tasks: &mut JoinSet<Option<TaskFailure>>, task: &'static str, future: F)
where
    F: Future<Output = Result<()>> + Send + 'static,
{
    tasks.spawn(async move {
        match AssertUnwindSafe(future).catch_unwind().await {
            Ok(Ok(())) => None,
            Ok(Err(error)) => Some(TaskFailure::message(task, error_detail(error))),
            Err(payload) => Some(TaskFailure::message(task, panic_detail(payload))),
        }
    });
}

pub(super) async fn restart_task<F, Fut, S>(
    task: &'static str,
    policy: RestartPolicy,
    mut operation: F,
    should_stop: S,
) where
    F: FnMut() -> Fut,
    Fut: Future<Output = Result<()>>,
    S: Fn() -> bool,
{
    let mut failure_count = 0_u32;
    while !should_stop() {
        let outcome = AssertUnwindSafe(async { operation().await })
            .catch_unwind()
            .await;
        if should_stop() {
            return;
        }
        failure_count = failure_count.saturating_add(1);
        let failure = match outcome {
            Ok(Ok(())) => TaskFailure::message(task, TASK_STOPPED_UNEXPECTEDLY),
            Ok(Err(error)) => TaskFailure::message(task, error_detail(error)),
            Err(payload) => TaskFailure::message(task, panic_detail(payload)),
        };
        let delay = report_restart(&failure, policy, failure_count);
        tokio::time::sleep(delay).await;
    }
}

pub(super) fn report_restart(
    failure: &TaskFailure,
    policy: RestartPolicy,
    failure_count: u32,
) -> std::time::Duration {
    let delay = policy.delay(failure_count);
    let _ = contain(|| {
        tracing::warn!(
            task = failure.task,
            detail = failure.detail,
            restart_delay_ms = delay.as_millis(),
            "contained worker task failure; restarting without terminating the PTY"
        );
    });
    delay
}

pub(super) fn report_contained(failure: &TaskFailure) {
    let _ = contain(|| {
        tracing::warn!(
            task = failure.task,
            detail = failure.detail,
            "contained auxiliary worker task failure; PTY session remains running"
        );
    });
}

pub(super) fn error_detail(error: anyhow::Error) -> String {
    let detail = error_ref_detail(&error);
    let _ = contain(|| drop(error));
    detail
}

pub(super) fn error_ref_detail(error: &anyhow::Error) -> String {
    contain(|| error.to_string()).unwrap_or_else(|| TASK_ERROR_DETAIL.to_owned())
}

pub(super) fn panic_detail(payload: Box<dyn Any + Send>) -> String {
    let detail = contain(|| {
        payload
            .downcast_ref::<&str>()
            .map(|value| (*value).to_owned())
            .or_else(|| payload.downcast_ref::<String>().cloned())
    })
    .flatten()
    .unwrap_or_else(|| TASK_PANIC_DETAIL.to_owned());
    let _ = contain(|| drop(payload));
    detail
}

pub(super) fn contain<T>(operation: impl FnOnce() -> T) -> Option<T> {
    match catch_unwind(AssertUnwindSafe(operation)) {
        Ok(value) => Some(value),
        Err(secondary) => {
            std::mem::forget(secondary);
            None
        }
    }
}

#[cfg(test)]
#[cfg(any(not(test), feature = "yolo-unit-tests"))]
#[cfg(any(feature = "yolo-integration-tests", feature = "yolo-worker-unit-runtime"))]
#[path = "../unit_tests/supervision/supervision_error_containment_tests.rs"]
mod error_containment_tests;
#[cfg(test)]
#[cfg(any(not(test), feature = "yolo-unit-tests"))]
#[cfg(any(feature = "yolo-integration-tests", feature = "yolo-worker-unit-runtime"))]
#[path = "../unit_tests/supervision/supervision_tests.rs"]
mod tests;

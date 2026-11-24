use super::{FakePending, PendingObservation, PendingSpawnHandle};
use anyhow::Result;
use std::future::Future;
use std::sync::{Arc, Mutex};

const TASK_EXIT_DETAIL: &str = "in-process broker task exited before readiness";
const TASK_FAILURE_PREFIX: &str = "in-process broker task failed before readiness: ";
const TASK_COMPLETION_MISSING: &str =
    "in-process broker task ended without publishing its completion";
const TASK_COMPLETION_LOCK_FAILURE: &str = "in-process broker task completion lock was poisoned";
const TASK_JOIN_FAILURE: &str = "in-process broker task join failed";
const TASK_START_MISSING: &str = "in-process broker task completed without its start handshake";

pub(crate) enum TrackedTaskSettlement {
    Completed(Result<()>),
    Cancelled,
}

pub(crate) struct TrackedBrokerTask {
    started: tokio::sync::oneshot::Receiver<()>,
    handle: tokio::task::JoinHandle<Result<()>>,
}

impl TrackedBrokerTask {
    pub(crate) fn abort(&self) {
        self.handle.abort();
    }

    pub(crate) async fn settle(self) -> Result<TrackedTaskSettlement> {
        let started = self.started.await.is_ok();
        match self.handle.await {
            Ok(result) => {
                anyhow::ensure!(started, TASK_START_MISSING);
                Ok(TrackedTaskSettlement::Completed(result))
            }
            Err(error) if error.is_cancelled() => Ok(TrackedTaskSettlement::Cancelled),
            Err(error) => anyhow::bail!("{TASK_JOIN_FAILURE}: {error}"),
        }
    }
}

impl FakePending {
    pub(crate) fn tracked_task<F>(future: F) -> (PendingSpawnHandle, TrackedBrokerTask)
    where
        F: Future<Output = Result<()>> + Send + 'static,
    {
        let completion = Arc::new(Mutex::new(None));
        let task_completion = Arc::clone(&completion);
        let (started_send, started) = tokio::sync::oneshot::channel();
        let task = tokio::spawn(async move {
            let _ = started_send.send(());
            let result = future.await;
            let detail = match &result {
                Ok(()) => TASK_EXIT_DETAIL.to_owned(),
                Err(error) => format!("{TASK_FAILURE_PREFIX}{error:#}"),
            };
            *task_completion.lock().expect(TASK_COMPLETION_LOCK_FAILURE) = Some(detail);
            result
        });
        let observer_task = task.abort_handle();
        let abort_task = observer_task.clone();
        let pending = Self::tracked_pending(
            move || {
                if !observer_task.is_finished() {
                    return Ok(PendingObservation::Pending);
                }
                let detail = completion
                    .lock()
                    .map_err(|_| anyhow::anyhow!(TASK_COMPLETION_LOCK_FAILURE))?
                    .clone()
                    .unwrap_or_else(|| TASK_COMPLETION_MISSING.to_owned());
                Ok(PendingObservation::Exited(detail))
            },
            move || {
                abort_task.abort();
                Ok(())
            },
        );
        (
            pending,
            TrackedBrokerTask {
                started,
                handle: task,
            },
        )
    }
}

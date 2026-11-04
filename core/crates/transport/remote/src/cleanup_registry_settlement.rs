use super::constants::CLEANUP_REGISTRY_TIMEOUT_MESSAGE;
use tokio::time::{Instant, timeout_at};

impl RemoteCleanupRegistry {
    pub async fn settle(&self, timeout: Duration) -> Result<(), RemoteError> {
        let _gate = self.inner.settlement_gate.lock().await;
        if let Some(result) = self.terminal_result() {
            return result;
        }
        self.inner.state.lock().unwrap().phase = RegistryPhase::Settling;
        let deadline = Instant::now() + timeout;
        loop {
            if let Some(task) = self.take_task() {
                if !self.finish_task(task, deadline).await {
                    return self.fail_timed_out();
                }
                continue;
            }
            let changed = self.inner.changed.notified();
            if let Some(result) = self.finish_if_idle() {
                return result;
            }
            #[cfg(test)]
            self.pause_before_changed_await_for_test().await;
            if timeout_at(deadline, changed).await.is_err() {
                return self.fail_timed_out();
            }
        }
    }

    fn take_task(&self) -> Option<CleanupTaskCustody<'_>> {
        self.inner
            .state
            .lock()
            .unwrap()
            .tasks
            .pop()
            .map(|task| CleanupTaskCustody {
                registry: self,
                task: Some(task),
            })
    }

    async fn finish_task(&self, mut task: CleanupTaskCustody<'_>, deadline: Instant) -> bool {
        match timeout_at(deadline, task.task()).await {
            Ok(Ok(Ok(()))) => {
                task.complete();
                true
            }
            Ok(Ok(Err(error))) => {
                tracing::error!(%error, "registered SSH cleanup failed");
                self.record_failure();
                task.complete();
                true
            }
            Ok(Err(error)) => {
                tracing::error!(%error, "registered SSH cleanup task failed");
                self.record_failure();
                task.complete();
                true
            }
            Err(_) => false,
        }
    }

    fn record_failure(&self) {
        let mut state = self.inner.state.lock().unwrap();
        state.failures = state
            .failures
            .saturating_add(SSH_CLEANUP_RESERVATION_INCREMENT);
    }

    fn finish_if_idle(&self) -> Option<CleanupResult> {
        let mut state = self.inner.state.lock().unwrap();
        if state.reservations != 0 || !state.tasks.is_empty() {
            return None;
        }
        if state.failures == 0 {
            state.phase = RegistryPhase::Complete;
            Some(Ok(()))
        } else {
            state.phase = RegistryPhase::Failed;
            Some(Err(RemoteError::transport(CLEANUP_REGISTRY_FAILED_MESSAGE)))
        }
    }

    fn fail_timed_out(&self) -> CleanupResult {
        let mut state = self.inner.state.lock().unwrap();
        state.failures = state
            .failures
            .saturating_add(SSH_CLEANUP_RESERVATION_INCREMENT);
        state.phase = RegistryPhase::Failed;
        Err(RemoteError::transport(CLEANUP_REGISTRY_TIMEOUT_MESSAGE))
    }
}

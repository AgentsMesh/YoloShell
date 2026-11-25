use std::sync::Mutex;

#[derive(Default)]
struct BrokerTasks {
    handles: Mutex<Vec<tokio::task::JoinHandle<()>>>,
}

impl BrokerTasks {
    async fn finish_with_hub_or_external<F>(
        &self,
        hub: &BrokerTasks,
        shutdown: std::pin::Pin<&mut F>,
    ) where
        F: Future<Output = ()>,
    {
        let mut combined = self.take();
        // Keep both task groups alive while waiting for natural closure or external shutdown.
        let mut hub_tasks = hub.take();
        {
            let draining = async {
                while let Some(task) = combined.current() {
                    let _ = task.await;
                    combined.complete_current();
                }
                while let Some(task) = hub_tasks.current() {
                    let _ = task.await;
                    hub_tasks.complete_current();
                }
            };
            tokio::pin!(draining);
            tokio::select! { _ = &mut draining => return, _ = shutdown => {} }
        }
        let deadline = tokio::time::Instant::now() + EXTERNAL_RUNTIME_DRAIN_TIMEOUT;
        tokio::join!(
            finish_before(&mut combined, deadline, RUNTIME_LISTENER_LABEL),
            finish_before(
                &mut hub_tasks,
                deadline,
                super::hub_peer_constants::HUB_TUNNEL_LABEL
            )
        );
    }
    fn spawn(&self, future: impl Future<Output = ()> + Send + 'static) {
        self.handles.lock().unwrap().push(tokio::spawn(future));
    }

    async fn finish_bounded(&self, timeout: std::time::Duration, label: &'static str) -> bool {
        let mut tasks = self.take();
        let deadline = tokio::time::Instant::now() + timeout;
        finish_before(&mut tasks, deadline, label).await
    }

    async fn finish_before(&self, deadline: tokio::time::Instant, label: &'static str) -> bool {
        let mut tasks = self.take();
        finish_before(&mut tasks, deadline, label).await
    }

    async fn finish_naturally_or_external<F>(&self, mut shutdown: std::pin::Pin<&mut F>)
    where
        F: Future<Output = ()>,
    {
        let mut tasks = self.take();
        while tasks.current().is_some() {
            let externally_stopped = {
                let task = tasks.current().expect(TASK_DRAIN_CURRENT_REQUIRED);
                tokio::select! {
                    _ = shutdown.as_mut() => true,
                    _ = task => false,
                }
            };
            if externally_stopped {
                let deadline = tokio::time::Instant::now() + EXTERNAL_RUNTIME_DRAIN_TIMEOUT;
                let _ = finish_before(&mut tasks, deadline, RUNTIME_LISTENER_LABEL).await;
                return;
            }
            tasks.complete_current();
        }
    }

    fn take(&self) -> TaskDrain {
        TaskDrain::new(std::mem::take(&mut *self.handles.lock().unwrap()))
    }
}

impl Drop for BrokerTasks {
    fn drop(&mut self) {
        let handles = self
            .handles
            .get_mut()
            .unwrap_or_else(|error| error.into_inner());
        handles.iter().for_each(tokio::task::JoinHandle::abort);
    }
}

async fn finish_before(
    tasks: &mut TaskDrain,
    deadline: tokio::time::Instant,
    label: &'static str,
) -> bool {
    while tasks.current().is_some() {
        let finished = {
            let task = tasks.current().expect(TASK_DRAIN_CURRENT_REQUIRED);
            tokio::time::timeout_at(deadline, task).await.is_ok()
        };
        if !finished {
            tracing::debug!(task_group = label, "broker task drain expired; canceling");
            return abort_and_settle(tasks).await;
        }
        tasks.complete_current();
    }
    true
}

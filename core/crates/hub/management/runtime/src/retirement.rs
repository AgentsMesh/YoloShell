use crate::{ManagementService, constants::GENERATION_RETAINED, generation::Running};
use anyhow::Result;
use std::sync::Arc;
use tokio::{task::JoinHandle, time::Instant};
use yolo_hub_federation::{Federation, STOP_TIMEOUT};

const MISSING_COMPLETION: &str = "retiring Hub generation has no network completion";

/// A timeout or cancelled waiter retains both network completion and resource custody.
pub(crate) struct Retiring {
    node: Arc<Federation>,
    task: Option<JoinHandle<Result<()>>>,
    completion: Option<Result<()>>,
}

impl Retiring {
    async fn wait(&mut self) -> Result<()> {
        if let Some(task) = &mut self.task {
            let completed = task
                .await
                .map_err(anyhow::Error::from)
                .and_then(|result| result);
            self.task.take();
            self.completion = Some(completed);
        }
        self.node.wait_retired(Instant::now() + STOP_TIMEOUT).await
    }
}

impl From<Running> for Retiring {
    fn from(running: Running) -> Self {
        Self {
            node: running.node,
            task: Some(running.task),
            completion: None,
        }
    }
}

impl ManagementService {
    pub(crate) async fn retire(&mut self) -> Result<()> {
        if let Some(running) = self.running.take() {
            running.node.begin_shutdown();
            running.stop.send_replace(true);
            self.retiring = Some(running.into());
        }
        // Retry this step too if a previous waiter was cancelled while awaiting the lock.
        self.hub.generation.write().await.take();
        let Some(retiring) = &mut self.retiring else {
            return Ok(());
        };
        retiring.wait().await?;
        anyhow::ensure!(Arc::strong_count(&retiring.node) == 1, GENERATION_RETAINED);
        self.history = Some(retiring.node.history().await);
        let retired = self.retiring.take().expect(MISSING_COMPLETION);
        retired.completion.expect(MISSING_COMPLETION)
    }
}

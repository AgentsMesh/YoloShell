use super::constants::{
    CODEX_PROJECTION_COMPLETION_CLOSED, CODEX_PROJECTION_COMPLETION_POISONED,
    CODEX_PROJECTION_DRAIN_ERROR, CODEX_PROJECTION_DRAIN_TIMEOUT, CODEX_PROJECTION_STARTED,
};
use anyhow::{Context, Result, ensure};
use futures::{
    FutureExt,
    future::{BoxFuture, Shared},
};
use std::sync::{Arc, Mutex};
use std::time::Duration;

type ProjectionResult = std::result::Result<(), Arc<str>>;
type SharedCompletion = Shared<BoxFuture<'static, ProjectionResult>>;

#[derive(Default)]
pub(super) struct ProjectionCompletion {
    receiver: Mutex<Option<SharedCompletion>>,
}

pub(super) struct ProjectionTicket(tokio::sync::oneshot::Sender<ProjectionResult>);

impl ProjectionCompletion {
    pub(super) fn start(&self) -> Result<ProjectionTicket> {
        let mut slot = self
            .receiver
            .lock()
            .map_err(|_| anyhow::anyhow!(CODEX_PROJECTION_COMPLETION_POISONED))?;
        ensure!(slot.is_none(), CODEX_PROJECTION_STARTED);
        let (sender, receiver) = tokio::sync::oneshot::channel();
        *slot = Some(
            async move {
                receiver
                    .await
                    .unwrap_or_else(|_| Err(Arc::from(CODEX_PROJECTION_COMPLETION_CLOSED)))
            }
            .boxed()
            .shared(),
        );
        Ok(ProjectionTicket(sender))
    }

    pub(super) async fn wait(&self) -> Result<()> {
        self.wait_for(CODEX_PROJECTION_DRAIN_TIMEOUT).await
    }

    async fn wait_for(&self, timeout: Duration) -> Result<()> {
        let receiver = self
            .receiver
            .lock()
            .map_err(|_| anyhow::anyhow!(CODEX_PROJECTION_COMPLETION_POISONED))?
            .clone();
        let Some(receiver) = receiver else {
            return Ok(());
        };
        tokio::time::timeout(timeout, receiver)
            .await
            .context(CODEX_PROJECTION_DRAIN_ERROR)?
            .map_err(|detail| anyhow::anyhow!(detail.to_string()))
    }
}

impl ProjectionTicket {
    pub(super) fn finish(self, outcome: &Result<()>) {
        let result = outcome
            .as_ref()
            .map(|_| ())
            .map_err(|error| Arc::from(crate::worker::supervision::error_ref_detail(error)));
        let _ = self.0.send(result);
    }
}

#[cfg(all(test, feature = "yolo-unit-tests"))]
#[cfg(any(feature = "yolo-integration-tests", feature = "yolo-worker-unit-runtime"))]
#[path = "../../../unit_tests/codex_hook/projection/projection_completion_tests.rs"]
mod tests;

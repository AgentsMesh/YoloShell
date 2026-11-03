use crate::worker::supervision;
use anyhow::{Context, Result};
use tokio::sync::oneshot;
use tokio::task::JoinSet;

const READINESS_ALREADY_CONSUMED: &str = "Claude hook admission readiness was already consumed";
const READINESS_DROPPED: &str = "Claude hook admission stopped before handler readiness";
const ACTIVATION_ALREADY_CONSUMED: &str = "Claude hook runtime activation was already consumed";
const ACTIVATION_RECEIVER_DROPPED: &str = "Claude hook admission stopped before runtime activation";

pub(in crate::worker) struct ClaudeHookAdmissionPump {
    readiness: Option<oneshot::Receiver<()>>,
    activation: Option<oneshot::Sender<()>>,
}

impl ClaudeHookAdmissionPump {
    pub(super) fn new(readiness: oneshot::Receiver<()>, activation: oneshot::Sender<()>) -> Self {
        Self {
            readiness: Some(readiness),
            activation: Some(activation),
        }
    }

    pub(in crate::worker) async fn wait_ready(&mut self) -> Result<()> {
        self.readiness
            .take()
            .context(READINESS_ALREADY_CONSUMED)?
            .await
            .context(READINESS_DROPPED)
    }

    pub(in crate::worker) fn retain_readiness(
        &mut self,
        tasks: &mut JoinSet<Option<supervision::TaskFailure>>,
    ) -> Result<()> {
        let readiness = self.readiness.take().context(READINESS_ALREADY_CONSUMED)?;
        supervision::spawn_task(
            tasks,
            super::constants::HOOK_READINESS_RETENTION_TASK,
            async move {
                let _ = readiness.await;
                Ok(())
            },
        );
        Ok(())
    }

    pub(in crate::worker) fn activate(mut self) -> Result<()> {
        self.activation
            .take()
            .context(ACTIVATION_ALREADY_CONSUMED)?
            .send(())
            .map_err(|_| anyhow::anyhow!(ACTIVATION_RECEIVER_DROPPED))
    }
}

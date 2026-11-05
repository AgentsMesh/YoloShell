use crate::{
    LocalBackend,
    connector_catalogue::ObservationScope,
    native_diagnostics::ObservationStage,
};
use futures::StreamExt;
use std::future::Future;
use std::time::Duration;
use tokio::sync::MutexGuard;
use yolo_agent_control_contract::NativeControlObservation;
use yolo_hub_agent_protocol::AgentSnapshot;
use yolo_hub_connector_registry::Catalogue;
use yolo_hub_identity::TerminalId;
use yolo_hub_rpc_protocol::Reply;
use yolo_terminal_model::TerminalView;

/// A Worker that cannot answer within this budget contributes no capability:
/// observations are advisory evidence for the read path, so a stalled Worker
/// must never hold a catalogue read open.
pub(crate) const NATIVE_OBSERVATION_TIMEOUT: Duration = Duration::from_secs(1);
/// The Host bounds every Worker status call itself; this only trips when the
/// Host has stopped answering. Without it one stalled read would hold the
/// sweep gate for the client's whole RPC budget.
pub(crate) const CATALOGUE_READ_TIMEOUT: Duration = Duration::from_secs(2);
pub(crate) const MAX_OBSERVATION_CONCURRENCY: usize = 16;

/// How a failed Host catalogue read affects the projection.
#[derive(Clone, Copy)]
pub(crate) enum SweepPolicy {
    /// Agent reads: unreachable rows stay listed, and a complete sweep forgets
    /// sessions it no longer observes.
    ObserveAgents(Option<TerminalId>),
    /// Catalogue reads report the Host failure to the caller.
    Catalogue,
}

/// One Host catalogue read and the observations gathered for it, collected
/// before the projection lock is taken.
pub(crate) struct ObservationSweep {
    pub(crate) views: Result<Vec<TerminalView>, yolo_client::ClientError>,
    pub(crate) native: Vec<Result<NativeControlObservation, ObservationStage>>,
    pub(crate) connectors: Catalogue,
}

pub(crate) struct AppliedCatalogue {
    pub(crate) views: Result<Vec<TerminalView>, yolo_client::ClientError>,
    pub(crate) projected: Vec<Option<AgentSnapshot>>,
    pub(crate) agents: Vec<AgentSnapshot>,
    pub(crate) connectors: Catalogue,
}

impl LocalBackend {
    /// `current` is advisory; an unavailable projection degrades to no
    /// annotation instead of failing the durable run read.
    pub(crate) async fn annotate_run_reply(&self, mut reply: Reply) -> Result<Reply, yolo_hub_terminal_protocol::PublicError> {
        if matches!(&reply, Reply::AgentRun { .. } | Reply::AgentRuns { .. }) {
            reply.annotate_run_current(&self.observed_targets().await);
        }
        Ok(reply)
    }

    /// Live terminal-to-execution joins from the projection and connectors.
    pub(crate) async fn observed_targets(&self) -> Vec<(TerminalId, yolo_hub_identity::AgentTarget)> {
        let mut targets = match &self.agents {
            Some(projection) => projection.lock().await.active_targets().unwrap_or_default(),
            None => Vec::new(),
        };
        let native: std::collections::BTreeSet<_> = targets.iter().map(|(terminal, _)| *terminal).collect();
        if let Some(registry) = &self.connectors
            && let Ok(catalogue) = registry.lock().await.catalogue()
        {
            targets.extend(catalogue.active_targets().into_iter().filter(|(terminal, _)| !native.contains(terminal)));
        }
        targets
    }

    /// Serializes sweeps so observations are applied in observation order.
    /// Readers of the projection never take this gate, and no Host or Worker
    /// I/O runs while the projection lock is held.
    pub(crate) async fn begin_observation(&self) -> MutexGuard<'_, ()> {
        self.observation_gate.lock().await
    }

    /// Bound every Host read taken while holding the sweep gate.
    pub(crate) async fn bounded_host_read<T>(
        &self,
        future: impl Future<Output = Result<T, yolo_client::ClientError>>,
    ) -> Result<T, yolo_client::ClientError> {
        match tokio::time::timeout(CATALOGUE_READ_TIMEOUT, future).await {
            Ok(result) => result,
            Err(_) => Err(yolo_client::ClientError::Deadline),
        }
    }

    /// Gathers the catalogue, the connector reconciliation and the native
    /// observations. The connector gate orders admission against the Host
    /// read; it is released before the projection is touched. The Host read
    /// and the observation phase share one envelope, so a catalogue read
    /// queued behind a maximal sweep still finishes inside the client budget.
    pub(crate) async fn observe_catalogue(
        &self,
        scope: ObservationScope,
        include_archived: bool,
    ) -> ObservationSweep {
        let deadline = tokio::time::Instant::now() + CATALOGUE_READ_TIMEOUT;
        let (views, connectors) = {
            let _connector_observation = self.connector_observation().await;
            let views = match scope {
                ObservationScope::Complete => {
                    self.bounded_host_read(self.catalogue_views(include_archived))
                        .await
                }
                ObservationScope::Terminal(address) => self
                    .bounded_host_read(self.client.get_session(address.terminal_id.as_uuid()))
                    .await
                    .map(|view| vec![view]),
            };
            let connectors = self
                .connector_catalogue(views.as_deref().ok(), scope)
                .await;
            (views, connectors)
        };
        let native = match &views {
            Ok(views) => self.capture_native(views, deadline).await,
            Err(_) => Vec::new(),
        };
        ObservationSweep {
            views,
            native,
            connectors,
        }
    }

    /// Captures observations concurrently and with one shared budget. An idle
    /// binding reuses its stored witness instead of asking the Worker again.
    async fn capture_native(
        &self,
        views: &[TerminalView],
        sweep_deadline: tokio::time::Instant,
    ) -> Vec<Result<NativeControlObservation, ObservationStage>> {
        let Some(projection) = &self.agents else {
            return views
                .iter()
                .map(|_| Err(ObservationStage::CapabilityUnavailable))
                .collect();
        };
        let reusable: Vec<Option<NativeControlObservation>> = {
            let projection = projection.lock().await;
            views
                .iter()
                .map(|view| projection.reusable_idle_observation(view).ok().flatten())
                .collect()
        };
        let deadline = (tokio::time::Instant::now() + NATIVE_OBSERVATION_TIMEOUT)
            .min(sweep_deadline);
        let captures = views
            .iter()
            .cloned()
            .zip(reusable)
            .map(|(view, reusable)| async move {
                if let Some(observation) = reusable {
                    return Ok(observation);
                }
                // One absolute budget for the whole phase: a stalled Worker
                // cannot stretch the sweep by queueing more terminals.
                match tokio::time::timeout_at(deadline, self.classify_native_observation(&view))
                    .await
                {
                    Ok(result) => result,
                    Err(_) => Err(ObservationStage::WorkerDeadline),
                }
            });
        futures::stream::iter(captures)
            .buffered(MAX_OBSERVATION_CONCURRENCY)
            .collect()
            .await
    }
}

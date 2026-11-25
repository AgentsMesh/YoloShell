use super::{AgentRunService, StoreError};
use futures::StreamExt;
use std::collections::{BTreeMap, BTreeSet};
use std::time::Duration;
use yolo_client::Client;
use yolo_hub_agent_run_protocol::{AgentRunSelector, AgentRunSummary};
use yolo_hub_agent_run_store::TerminalObservation;
use yolo_hub_identity::{HubId, TerminalAddress, TerminalId};
use yolo_hub_terminal_protocol::PublicError;
use yolo_terminal_model::{TerminalRuntimeStatus, TerminalView};

const ARCHIVE_LOOKUP_CONCURRENCY: usize = 16;
const ARCHIVE_LOOKUP_BUDGET: Duration = Duration::from_millis(500);

pub(crate) fn run_name(provider_id: &str, start_token: uuid::Uuid) -> String {
    let suffix = &start_token.simple().to_string()[..8];
    format!("{provider_id}-{suffix}")
}

impl AgentRunService {
    pub(crate) async fn stop_target(
        &self,
        selector: &AgentRunSelector,
    ) -> Result<(AgentRunSummary, Option<uuid::Uuid>, Option<uuid::Uuid>), PublicError> {
        self.store
            .lock()
            .await
            .stop_target(selector)
            .map_err(StoreError::into_public)?
            .ok_or(PublicError::NotFound)
    }

    pub(crate) async fn observe_catalogue(
        &self,
        client: &Client,
        hub: HubId,
        views: &[TerminalView],
        includes_archived: bool,
    ) -> Result<(), PublicError> {
        let mut observations = BTreeMap::new();
        let mut present = BTreeSet::new();
        for view in views {
            let terminal_id = TerminalId::from_uuid(view.manifest.terminal_id)
                .map_err(|_| PublicError::Internal)?;
            let address = TerminalAddress {
                hub_id: hub,
                terminal_id,
            };
            present.insert(address);
            if let Some(observation) = terminal_observation(view) {
                observations.insert(address, observation);
            }
        }
        if !includes_archived {
            // The live catalogue omits archived terminals. Look up only
            // missing launched runs, within one budget for the entire batch.
            let launched = self
                .store
                .lock()
                .await
                .launched_terminals()
                .map_err(StoreError::into_public)?;
            let deadline = tokio::time::Instant::now() + ARCHIVE_LOOKUP_BUDGET;
            let lookups = launched
                .difference(&present)
                .copied()
                .map(|address| async move {
                    let view = tokio::time::timeout_at(
                        deadline,
                        client.get_session(address.terminal_id.as_uuid()),
                    )
                    .await
                    .ok()?
                    .ok()?;
                    if view.manifest.terminal_id != address.terminal_id.as_uuid() {
                        return None;
                    }
                    terminal_observation(&view).map(|observation| (address, observation))
                });
            let mut omitted =
                futures::stream::iter(lookups).buffer_unordered(ARCHIVE_LOOKUP_CONCURRENCY);
            while let Some(observation) = omitted.next().await {
                observations.extend(observation);
            }
        }
        let ended: Vec<_> = observations
            .iter()
            .filter_map(|(address, observation)| {
                matches!(observation, TerminalObservation::Ended(_)).then_some(*address)
            })
            .collect();
        let deadline = tokio::time::Instant::now() + ARCHIVE_LOOKUP_BUDGET;
        let fresh = futures::stream::iter(ended.into_iter().map(|address| async move {
            let view = tokio::time::timeout_at(
                deadline,
                client.get_session(address.terminal_id.as_uuid()),
            )
            .await
            .ok()?
            .ok()?;
            terminal_observation(&view).map(|observation| (address, observation))
        }))
        .buffer_unordered(ARCHIVE_LOOKUP_CONCURRENCY)
        .collect::<Vec<_>>()
        .await;
        for observation in fresh.into_iter().flatten() {
            observations.insert(observation.0, observation.1);
        }
        self.store
            .lock()
            .await
            .observe_terminals(&observations)
            .map_err(StoreError::into_public)
    }
}

pub(crate) fn terminal_observation(view: &TerminalView) -> Option<TerminalObservation> {
    if view.manifest.archived {
        return Some(TerminalObservation::Archived);
    }
    let runtime = view.runtime.as_ref()?;
    if runtime.runtime_epoch.is_nil() {
        return None;
    }
    match runtime.status {
        TerminalRuntimeStatus::Starting
        | TerminalRuntimeStatus::Running
        | TerminalRuntimeStatus::Finalizing
        | TerminalRuntimeStatus::OutputBlocked => {
            Some(TerminalObservation::Live(runtime.runtime_epoch))
        }
        TerminalRuntimeStatus::Exited | TerminalRuntimeStatus::Lost => {
            Some(TerminalObservation::Ended(runtime.runtime_epoch))
        }
        TerminalRuntimeStatus::Unreachable => None,
    }
}

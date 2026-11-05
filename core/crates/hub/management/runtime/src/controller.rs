use crate::{
    ManagedHub, ManagementRequest, ManagementResult,
    constants::*,
    generation::{self, Running},
};
use anyhow::Result;
use std::{path::PathBuf, sync::Arc};
use tokio::sync::{mpsc, watch};
use yolo_hub_configuration_store::ConfigurationStore;
use yolo_hub_management_protocol::{ConfigurationSnapshot, ConfigurationState};
use yolo_hub_terminal_protocol::PublicError;

pub struct ManagementService {
    pub(crate) hub: Arc<ManagedHub>,
    pub(crate) receiver: mpsc::Receiver<ManagementRequest>,
    pub(crate) store: ConfigurationStore,
    pub(crate) backend: yolo_hub_local_backend::LocalBackend,
    pub(crate) listener: crate::listener::PeerListener,
    pub(crate) running: Option<Running>,
    pub(crate) retiring: Option<crate::retirement::Retiring>,
    pub(crate) configuration: ConfigurationState,
    pub(crate) history: Option<yolo_hub_federation::ObservationHistory>,
    pub(crate) activation_pending: bool,
    pub(crate) origin: Option<Arc<dyn yolo_hub_dispatch::OriginDecorator<yolo_hub_local_backend::LocalBackend>>>,
}

impl ManagementService {
    pub fn peer_socket(&self) -> Result<PathBuf> {
        self.hub.state.peer_socket()
    }

    pub async fn run(mut self, mut shutdown: watch::Receiver<bool>) -> Result<()> {
        let mut verification = tokio::time::interval(CONFIGURATION_VERIFY_INTERVAL);
        let mut activation_retry = tokio::time::interval(ACTIVATION_RETRY_INTERVAL);
        // The retry branch is disabled while activation is healthy. Burst
        // catch-up would replay every missed tick after a long uptime.
        activation_retry.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
        let result = loop {
            if *shutdown.borrow() {
                break Ok(());
            }
            tokio::select! {
                _ = shutdown.changed() => break Ok(()),
                _ = generation::failure(&mut self.running) => break Err(anyhow::anyhow!(GENERATION_FAILED)),
                _ = verification.tick() => {
                    if self.snapshot().is_err() { break Err(anyhow::anyhow!(CONFIGURATION_LOST)); }
                }
                _ = activation_retry.tick(), if self.activation_pending => {
                    self.retry_activation().await;
                }
                request = self.receiver.recv() => {
                    let Some(request) = request else { break Ok(()); };
                    let mutates = request.change.is_some();
                    let pending_before = self.activation_pending;
                    let result = match request.change {
                        Some(change) => self.change(change).await,
                        None => self.checked_snapshot().await,
                    };
                    let _ = request.reply.send(result);
                    if reset_activation_retry_window(
                        pending_before,
                        mutates,
                        self.activation_pending,
                    ) {
                        // Start the retry window at the failure, not at a tick
                        // consumed before activation became pending. Requests
                        // arriving while already pending must not postpone it.
                        activation_retry.reset();
                    }
                }
            }
        };
        self.receiver.close();
        let stopped = self.retire().await;
        result?;
        stopped
    }

    pub(crate) async fn checked_snapshot(&mut self) -> ManagementResult {
        let result = self.snapshot();
        if result.is_err() {
            let _ = self.retire().await;
        }
        result
    }

    pub(crate) fn snapshot(&self) -> ManagementResult {
        let state = self.store.current().map_err(|_| PublicError::Unreachable)?;
        let applied = self
            .running
            .as_ref()
            .is_some_and(|running| !*running.finished.borrow())
            && state == self.configuration;
        let peer_socket = if applied {
            Some(self.peer_socket().map_err(|_| PublicError::Internal)?)
        } else {
            None
        };
        let snapshot = ConfigurationSnapshot {
            state,
            applied,
            peer_socket,
        };
        snapshot.validate().map_err(|_| PublicError::Internal)?;
        Ok(snapshot)
    }
}

impl Drop for ManagementService {
    fn drop(&mut self) {
        if let Some(running) = &self.running {
            running.node.begin_shutdown();
            running.stop.send_replace(true);
        }
    }
}
/// Only the transition into pending starts a retry window; later failing
/// requests must not postpone the autonomous retry.
fn reset_activation_retry_window(pending_before: bool, mutates: bool, pending_after: bool) -> bool {
    mutates && !pending_before && pending_after
}

#[cfg(all(test, feature = "yolo-unit-tests"))]
#[path = "../unit_tests/controller_retry_tests.rs"]
mod tests;

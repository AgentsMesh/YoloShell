use crate::{
    ManagedHub, ManagementRequest, ManagementResult,
    constants::*,
    generation::{self, Running},
    invites::InviteLedger,
};
use anyhow::Result;
use std::{
    net::{SocketAddr, TcpListener},
    sync::Arc,
};
use tokio::sync::{mpsc, watch};
use yolo_hub_configuration_store::ConfigurationStore;
use yolo_hub_enrollment_server::{EnrollmentAdmission, EnrollmentService};
use yolo_hub_local_backend::LocalBackend;
use yolo_hub_management_protocol::{ConfigurationSnapshot, ConfigurationState};
use yolo_hub_terminal_protocol::PublicError;

pub struct ManagementService {
    pub(crate) hub: Arc<ManagedHub>,
    pub(crate) receiver: mpsc::Receiver<ManagementRequest>,
    pub(crate) enrollments: mpsc::Receiver<EnrollmentAdmission>,
    pub(crate) enrollment: EnrollmentService,
    pub(crate) invites: InviteLedger,
    pub(crate) store: ConfigurationStore,
    pub(crate) bindings: yolo_hub_listener_binding::ListenerBindings,
    pub(crate) backend: LocalBackend,
    pub(crate) listener: Option<TcpListener>,
    pub(crate) running: Option<Running>,
    pub(crate) retiring: Option<crate::retirement::Retiring>,
    pub(crate) configuration: ConfigurationState,
    pub(crate) history: Option<yolo_hub_federation::ObservationHistory>,
    pub(crate) activation_pending: bool,
    pub(crate) origin: Option<Arc<dyn yolo_hub_dispatch::OriginDecorator<LocalBackend>>>,
}

impl ManagementService {
    pub fn listen_address(&self) -> Result<Option<SocketAddr>> {
        crate::listener::address(&self.listener)
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
                admission = self.enrollments.recv() => {
                    let Some(admission) = admission else { break Ok(()); };
                    self.admit_enrollment(admission).await;
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
            self.listener = None;
        }
        result
    }

    pub(crate) fn snapshot(&self) -> ManagementResult {
        let state = self.store.current().map_err(|_| PublicError::Unreachable)?;
        self.bindings
            .verify(&state)
            .map_err(|_| PublicError::Unreachable)?;
        let applied = self
            .running
            .as_ref()
            .is_some_and(|running| !*running.finished.borrow())
            && state == self.configuration;
        let listen_address = if applied {
            crate::listener::verify(&self.bindings, &state, &self.listener)
                .map_err(|_| PublicError::Unreachable)?;
            self.listen_address().map_err(|_| PublicError::Internal)?
        } else {
            None
        };
        let snapshot = ConfigurationSnapshot {
            state,
            applied,
            listen_address,
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

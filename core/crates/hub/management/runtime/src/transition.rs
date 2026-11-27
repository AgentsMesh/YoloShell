use crate::{ManagementResult, ManagementService, constants::*, generation::Running};
use yolo_hub_configuration_store::{RevisionConflict, RevisionExhausted};
use yolo_hub_management_protocol::ConfigurationChange;
use yolo_hub_terminal_protocol::PublicError;

impl ManagementService {
    pub(crate) async fn change(&mut self, change: ConfigurationChange) -> ManagementResult {
        change
            .validate(self.hub.state.peer_identity())
            .map_err(|_| PublicError::InvalidRequest)?;
        let current = match self.store.check(&change) {
            Ok(current) => current,
            Err(error) if error.is::<RevisionConflict>() || error.is::<RevisionExhausted>() => {
                return Err(PublicError::Conflict);
            }
            Err(_) => {
                let _ = self.retire().await;
                return Err(PublicError::Unreachable);
            }
        };
        if current.last_change == change.request_id && self.running.is_some() {
            return self.confirmed_snapshot().await;
        }
        let directory = self
            .hub
            .state
            .directory()
            .map_err(|_| PublicError::Unreachable)?;
        let prepared_backend = self.backend
            .prepare_files(directory.path(), &change.configuration.file_roots)
            .await?;
        self.retire()
            .await
            .map_err(|_| PublicError::OutcomeUnknown)?;
        let state = match self.store.apply(change) {
            Ok(state) => state,
            Err(error) => {
                tracing::warn!(%error, message = UPDATE_FAILED);
                // The previous generation was already retired. A write
                // failure marks the store lost, and that handle can never
                // activate again; the controller's verification tick retires
                // the service. A readable store (revision exhaustion) can
                // still retry activation autonomously.
                self.activation_pending = !self.store.is_lost();
                return Err(PublicError::OutcomeUnknown);
            }
        };
        self.configuration = state;
        match self.start_generation(prepared_backend).await {
            Ok(()) => self.confirmed_snapshot().await,
            Err(error) => {
                tracing::warn!(%error, message = UPDATE_FAILED);
                // The configuration is already durable. Keep trying so a
                // transient activation failure cannot leave federation off
                // until an unrelated management request arrives.
                self.activation_pending = true;
                Err(PublicError::OutcomeUnknown)
            }
        }
    }

    async fn start_generation(&mut self, prepared_backend: yolo_hub_local_backend::LocalBackend) -> anyhow::Result<()> {
        let running = Running::start(
            self.hub.state.clone(),
            prepared_backend,
            self.configuration.configuration.clone(),
            &self.listener,
            self.history.clone(),
            self.hub.dialer.clone(),
            self.origin.clone(),
        )
        .await?;
        *self.hub.generation.write().await = Some(running.node.clone());
        self.running = Some(running);
        self.activation_pending = false;
        Ok(())
    }

    /// Retries activation of a configuration whose durable commit succeeded
    /// after a failed generation start. The running loop drives this while
    /// `activation_pending` is set.
    pub(crate) async fn retry_activation(&mut self) -> bool {
        if self.running.is_some() {
            self.activation_pending = false;
            return true;
        }
        if !self.activation_pending {
            return false;
        }
        let _current = match self.store.current() {
            Ok(current) if current == self.configuration => current,
            Ok(_) => {
                tracing::warn!(message = ACTIVATION_RETRY_FAILED);
                return false;
            }
            Err(error) => {
                tracing::warn!(%error, message = ACTIVATION_RETRY_FAILED);
                return false;
            }
        };
        let directory = match self.hub.state.directory() {
            Ok(directory) => directory,
            Err(error) => {
                tracing::warn!(%error, message = ACTIVATION_RETRY_FAILED);
                return false;
            }
        };
        let prepared_backend = match self.backend
            .prepare_files(
                directory.path(),
                &self.configuration.configuration.file_roots,
            )
            .await
        {
            Ok(prepared) => prepared,
            Err(error) => {
                tracing::warn!(%error, message = ACTIVATION_RETRY_FAILED);
                return false;
            }
        };
        match self.start_generation(prepared_backend).await {
            Ok(()) => {
                tracing::info!(message = ACTIVATION_RECOVERED);
                true
            }
            Err(error) => {
                tracing::warn!(%error, message = ACTIVATION_RETRY_FAILED);
                false
            }
        }
    }
    async fn confirmed_snapshot(&mut self) -> ManagementResult {
        let snapshot = self.checked_snapshot().await?;
        if snapshot.applied {
            Ok(snapshot)
        } else {
            Err(PublicError::OutcomeUnknown)
        }
    }
}

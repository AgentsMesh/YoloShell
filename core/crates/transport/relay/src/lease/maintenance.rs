use super::{LeaseController, is_ownership_error};
use crate::client::ClientError;
use crate::relay::RelayError;
use tokio::time::Instant;

impl LeaseController {
    pub(crate) fn maintenance_deadline(&self) -> Instant {
        self.next_maintenance
    }

    pub(crate) fn maintenance_due(&self) -> bool {
        !self.is_terminal() && Instant::now() >= self.next_maintenance
    }

    pub(crate) async fn maintain(&mut self) -> Result<(), RelayError> {
        if self.is_terminal() {
            return Ok(());
        }
        if let Some(grant) = self.lease.clone() {
            let result = self
                .runtime
                .client()
                .renew_writer(&grant, self.target, None)
                .await;
            match result {
                Ok(renewed) => self.set_writer(renewed)?,
                Err(error) if self.observe_session_ending(&error) => {}
                Err(error) if is_ownership_error(&error) => self.reconcile_recovering(true).await?,
                Err(error)
                    if super::super::client_error_is_retryable(&error)
                        || super::super::host::observation_interrupted(&error) =>
                {
                    self.reconnect_and_reconcile(true).await?;
                }
                Err(error) => return Err(error.into()),
            }
        } else if self.retry_mode.enabled() {
            self.retry_acquisition_recovering().await?;
        } else {
            self.schedule_read_only();
        }
        Ok(())
    }

    pub(crate) async fn mutation_rejected(
        &mut self,
        error: &ClientError,
    ) -> Result<bool, RelayError> {
        if self.observe_session_ending(error) {
            return Ok(true);
        }
        if self.is_terminal() {
            return Ok(false);
        }
        if !is_ownership_error(error) {
            return Ok(false);
        }
        self.reconcile_recovering(true).await?;
        Ok(true)
    }

    pub(crate) async fn recover_transport(&mut self) -> Result<(), RelayError> {
        if self.is_terminal() {
            return Ok(());
        }
        self.reconnect_and_reconcile(true).await
    }

    pub(crate) fn retain_control_intent(&mut self) {
        if !self.is_terminal() {
            self.retry_mode = self.retry_mode.retaining_intent(true);
        }
    }
}

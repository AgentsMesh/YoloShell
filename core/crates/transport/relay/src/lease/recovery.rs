use super::LeaseController;
use crate::relay::constants::CONTROL_RECONNECT_DELAY;
use crate::relay::{RelayError, relay_error_is_retryable};

impl LeaseController {
    pub(super) async fn reconcile_recovering(&mut self, lost: bool) -> Result<(), RelayError> {
        loop {
            if self.is_terminal() {
                return Ok(());
            }
            match self.reconcile(lost).await {
                Ok(()) => return Ok(()),
                Err(error) if relay_error_is_retryable(&error) => {
                    self.reconnect_runtime().await?;
                }
                Err(error) => return Err(error),
            }
        }
    }

    pub(super) async fn retry_acquisition_recovering(&mut self) -> Result<(), RelayError> {
        loop {
            if self.is_terminal() {
                return Ok(());
            }
            match self.retry_acquisition().await {
                Ok(()) => return Ok(()),
                Err(error) if relay_error_is_retryable(&error) => {
                    self.reconnect_runtime().await?;
                }
                Err(error) => return Err(error),
            }
        }
    }

    pub(super) async fn reconnect_and_reconcile(&mut self, lost: bool) -> Result<(), RelayError> {
        if self.is_terminal() {
            return Ok(());
        }
        self.reconnect_runtime().await?;
        self.reconcile_recovering(lost).await
    }

    async fn reconnect_runtime(&mut self) -> Result<(), RelayError> {
        loop {
            if self.is_terminal() {
                return Ok(());
            }
            tokio::time::sleep(CONTROL_RECONNECT_DELAY).await;
            if self.is_terminal() {
                return Ok(());
            }
            match self.runtime.reconnect(self.target.terminal_id).await {
                Ok(()) => return Ok(()),
                Err(error) if relay_error_is_retryable(&error) => {}
                Err(error) => return Err(error),
            }
        }
    }
}

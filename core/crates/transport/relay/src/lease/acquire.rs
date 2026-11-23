use super::super::constants::{TAKEOVER_MAX_ATTEMPTS, error_code};
use super::LeaseController;
use crate::client::ClientError;
use crate::model::WriterLeaseGrant;
use crate::relay::RelayError;

impl LeaseController {
    pub(super) async fn retry_acquisition(&mut self) -> Result<(), RelayError> {
        if self.is_terminal() {
            return Ok(());
        }
        match self.retry_mode {
            super::LeaseAcquireMode::Disabled => Ok(()),
            super::LeaseAcquireMode::Observe => self.acquire().await,
            super::LeaseAcquireMode::Takeover => self.takeover_pending().await,
        }
    }

    pub(super) async fn acquire(&mut self) -> Result<(), RelayError> {
        if self.is_terminal() {
            return Ok(());
        }
        self.ownership_uncertain = true;
        let result = self.request_acquire().await;
        if result
            .as_ref()
            .is_err_and(super::super::host::observation_interrupted)
        {
            self.runtime.reconnect(self.target.terminal_id).await?;
            return self.recover_uncertain_acquire().await;
        }
        self.finish_acquire(result, false).await
    }

    pub(super) async fn request_acquire(&self) -> Result<WriterLeaseGrant, ClientError> {
        self.runtime
            .client()
            .acquire_writer_for_instance(
                self.target,
                self.identity.holder_id.clone(),
                Some(self.identity.controller_instance_id.clone()),
                self.writer_capability.clone(),
                None,
            )
            .await
    }

    async fn recover_uncertain_acquire(&mut self) -> Result<(), RelayError> {
        if self.is_terminal() {
            return Ok(());
        }
        let current = self.runtime.client().get_writer_lease(self.target).await?;
        match current {
            Some(lease) if self.is_ours(&lease) => {
                let result = self.request_acquire().await;
                self.finish_acquire(result, true).await
            }
            Some(lease) => {
                self.ownership_uncertain = false;
                self.set_read_only(Some(lease.holder_id), true, true);
                Ok(())
            }
            None => {
                let result = self.request_acquire().await;
                self.finish_acquire(result, true).await
            }
        }
    }

    pub(super) async fn finish_acquire(
        &mut self,
        result: Result<WriterLeaseGrant, ClientError>,
        lost: bool,
    ) -> Result<(), RelayError> {
        if self.is_terminal() {
            return Ok(());
        }
        match result {
            Ok(grant) => self.set_writer(grant),
            Err(error) if self.observe_session_ending(&error) => Ok(()),
            Err(error) if is_conflict(&error) => {
                self.ownership_uncertain = false;
                let holder = self.current_holder().await?;
                self.set_read_only(holder, lost, true);
                Ok(())
            }
            Err(error) if super::is_ownership_error(&error) => {
                self.ownership_uncertain = false;
                let holder = self.current_holder().await?;
                self.set_read_only(holder, true, false);
                Ok(())
            }
            Err(error) => Err(error.into()),
        }
    }

    pub(super) async fn takeover(&mut self) -> Result<(), RelayError> {
        if self.is_terminal() {
            return Ok(());
        }
        self.retry_mode = super::LeaseAcquireMode::Takeover;
        self.takeover_pending().await
    }

    async fn takeover_pending(&mut self) -> Result<(), RelayError> {
        for _ in 0..TAKEOVER_MAX_ATTEMPTS {
            if self.is_terminal() {
                return Ok(());
            }
            let current = self.runtime.client().get_writer_lease(self.target).await?;
            let Some(lease) = current else {
                return self.acquire().await;
            };
            if self.is_ours(&lease) {
                return self.acquire().await;
            }
            self.ownership_uncertain = true;
            let result = self
                .runtime
                .client()
                .take_over_writer_for_instance(
                    self.target,
                    self.identity.holder_id.clone(),
                    Some(self.identity.controller_instance_id.clone()),
                    Some(lease.generation),
                    self.writer_capability.clone(),
                    None,
                )
                .await;
            match result {
                Ok(grant) => return self.set_writer(grant),
                Err(error) if self.observe_session_ending(&error) => return Ok(()),
                Err(error) if is_race(&error) => {
                    self.ownership_uncertain = false;
                }
                Err(error) if super::super::host::observation_interrupted(&error) => {
                    self.runtime.reconnect(self.target.terminal_id).await?;
                    return self.recover_uncertain_acquire().await;
                }
                Err(error) if super::super::client_error_is_retryable(&error) => {
                    return Err(error.into());
                }
                Err(error) => {
                    self.ownership_uncertain = false;
                    return Err(error.into());
                }
            }
        }
        Err(RelayError::TakeoverContended)
    }

    async fn current_holder(&self) -> Result<Option<String>, RelayError> {
        Ok(self
            .runtime
            .client()
            .get_writer_lease(self.target)
            .await?
            .map(|lease| lease.holder_id))
    }
}

fn is_conflict(error: &ClientError) -> bool {
    matches!(error, ClientError::Remote { code, .. } if code == error_code::HELD_BY_OTHER)
}

fn is_race(error: &ClientError) -> bool {
    matches!(error, ClientError::Remote { code, .. } if matches!(code.as_str(),
        error_code::HELD_BY_OTHER | error_code::STALE_GENERATION))
}

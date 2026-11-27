use super::LeaseController;
use crate::model::WriterLease;
use crate::relay::RelayError;

impl LeaseController {
    pub(super) async fn reconcile(&mut self, lost: bool) -> Result<(), RelayError> {
        if self.is_terminal() {
            return Ok(());
        }
        let held_before_query = self.lease.is_some();
        let current = self
            .runtime
            .client()
            .get_writer_lease(self.target)
            .await
            .map_err(crate::relay::host::observation_error)?;
        match current {
            Some(lease) if self.is_ours(&lease) => self.verify_public_lease(lease, lost).await,
            Some(lease) => {
                let retry = !held_before_query && self.retry_mode.enabled();
                self.set_read_only(Some(lease.holder_id), lost, retry);
                Ok(())
            }
            None => {
                let should_acquire = self.retry_mode.enabled() || held_before_query;
                self.set_read_only(None, lost, should_acquire);
                if should_acquire {
                    self.retry_acquisition().await?;
                }
                Ok(())
            }
        }
    }

    async fn verify_public_lease(
        &mut self,
        lease: WriterLease,
        lost: bool,
    ) -> Result<(), RelayError> {
        if self.is_terminal() {
            return Ok(());
        }
        let matching = self
            .lease
            .as_ref()
            .filter(|grant| grant.lease == lease)
            .cloned();
        if let Some(grant) = matching {
            let result = self
                .runtime
                .client()
                .renew_writer(&grant, self.target, None)
                .await;
            return match result {
                Ok(renewed) => self.set_writer(renewed),
                Err(error) if self.observe_session_ending(&error) => Ok(()),
                Err(error) if super::is_ownership_error(&error) => {
                    self.set_read_only(Some(lease.holder_id), true, false);
                    Ok(())
                }
                Err(error) if super::super::host::observation_interrupted(&error) => {
                    self.runtime.reconnect(self.target.terminal_id).await?;
                    self.verify_by_acquire(lease, lost).await
                }
                Err(error) => Err(error.into()),
            };
        }
        self.verify_by_acquire(lease, lost).await
    }

    async fn verify_by_acquire(
        &mut self,
        lease: WriterLease,
        lost: bool,
    ) -> Result<(), RelayError> {
        if self.is_terminal() {
            return Ok(());
        }
        self.ownership_uncertain = true;
        let result = self.request_acquire().await;
        match result {
            Ok(grant) => self.set_writer(grant),
            Err(error) if self.observe_session_ending(&error) => Ok(()),
            Err(error) if super::is_ownership_error(&error) => {
                self.ownership_uncertain = false;
                self.set_read_only(Some(lease.holder_id), lost, false);
                Ok(())
            }
            Err(error) => Err(error.into()),
        }
    }
}

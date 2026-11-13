use super::{PreparedSession, RelayError, RelayRuntime};
use crate::model::ExitRecord;
use uuid::Uuid;

impl RelayRuntime {
    pub(crate) async fn reconnect(&self, terminal_id: Uuid) -> Result<(), RelayError> {
        self.ensure_host().await?;
        if self.ensure_session_attachable(terminal_id).await? == PreparedSession::UserTerminated {
            return Err(RelayError::SessionNotAttachable(terminal_id));
        }
        Ok(())
    }

    pub(crate) async fn reject_user_terminated(&self, terminal_id: Uuid) -> Result<(), RelayError> {
        let view = self
            .client
            .get_session(terminal_id)
            .await
            .map_err(super::observation_error)?;
        if super::preparation::session_plan(&view)
            == super::preparation::SessionPlan::Attach(PreparedSession::UserTerminated)
        {
            return Err(RelayError::SessionNotAttachable(terminal_id));
        }
        Ok(())
    }

    pub(crate) async fn reconnect_transport(&self) -> Result<(), RelayError> {
        self.ensure_host().await?;
        Ok(())
    }

    pub(crate) async fn reconnect_completed(
        &self,
        terminal_id: Uuid,
        runtime_epoch: Uuid,
        expected_exit: &ExitRecord,
    ) -> Result<(), RelayError> {
        self.ensure_host().await?;
        self.ensure_same_completed_attachment(terminal_id, runtime_epoch, expected_exit)
            .await
    }
}

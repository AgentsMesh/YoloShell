use crate::ClientAccessRuntime;
use yolo_hub_client_access_protocol::{AccessConfiguration, ClientAccessChange};
use yolo_hub_terminal_protocol::PublicError;

impl ClientAccessRuntime {
    pub(crate) fn configure(
        &self,
        change: &ClientAccessChange,
    ) -> Result<AccessConfiguration, PublicError> {
        change.validate().map_err(|_| PublicError::InvalidRequest)?;
        if change.configuration.entry_hub != self.state.hub_id() {
            return Err(PublicError::OriginMismatch);
        }
        let mut store = self.lock_store().map_err(|_| PublicError::OutcomeUnknown)?;
        let current = self
            .fail_closed(self.current_locked(&store))
            .map_err(|_| PublicError::OutcomeUnknown)?;
        // Routine stale proposals are rejected before entering the poison-on-error store.
        if current.revision != change.expected_revision {
            return Err(PublicError::Conflict);
        }
        let written = (|| {
            let next = store.replace(change.expected_revision, change.configuration.clone())?;
            self.registry.replace(current.revision, next.clone())?;
            self.current_locked(&store)
        })();
        self.fail_closed(written)
            .map_err(|_| PublicError::OutcomeUnknown)
    }
}

use crate::{Federation, node::now_ms};
use yolo_hub_delegation::PeerIdentity;
use yolo_hub_peer_protocol::{Call, Signed, required_rights};
use yolo_hub_terminal_protocol::PublicError;

impl Federation {
    pub(crate) async fn admit_read_reply(
        &self,
        peer: PeerIdentity,
        call: &Signed<Call>,
    ) -> Result<(), PublicError> {
        if call
            .payload
            .request
            .fields()
            .command
            .requires_read_reply_admission()
        {
            self.admit(peer, call).await?;
        }
        Ok(())
    }

    pub(crate) async fn admit_entry_read_reply(
        &self,
        call: &Signed<Call>,
    ) -> Result<(), PublicError> {
        let request = call.payload.request.fields();
        if !request.command.requires_read_reply_admission() {
            return Ok(());
        }
        self.require_open().map_err(|_| PublicError::Unreachable)?;
        let now = now_ms().map_err(|_| PublicError::Internal)?;
        if now >= call.payload.expires_at_ms {
            return Err(PublicError::Unauthorized);
        }
        let book = self.book.lock().await;
        let pin = book
            .pin(request.destination)
            .ok_or(PublicError::Unauthorized)?;
        let current = book
            .latest_for(request.destination)
            .ok_or(PublicError::Unreachable)?;
        let Some(required) = required_rights(&request.command) else {
            return Err(PublicError::Unauthorized);
        };
        if book
            .route_for(request.destination, &required, now)
            .map_err(|_| PublicError::Internal)?
            .is_none()
        {
            return Err(PublicError::Unreachable);
        }
        call.payload
            .authority
            .authorize(
                pin,
                current.payload.policy_epoch,
                self.state.peer_identity(),
                now,
                &required,
            )
            .map_err(|_| PublicError::Unauthorized)?;
        self.require_open().map_err(|_| PublicError::Unreachable)
    }
}

#[cfg(all(test, feature = "yolo-unit-tests"))]
#[path = "../unit_tests/read_reply_admission.rs"]
mod unit_tests;

use super::LeaseController;
use crate::lease::{LeaseEventPayload, LeaseReason};
use crate::protocol::{EventEnvelope, event, from_value};
use crate::relay::RelayError;

impl LeaseController {
    pub(crate) fn observe_event(&mut self, envelope: &EventEnvelope) -> Result<(), RelayError> {
        let relevant = matches!(
            envelope.event.as_str(),
            event::WRITER_LEASE_ACQUIRED
                | event::WRITER_LEASE_REVOKED
                | event::WRITER_LEASE_EXPIRED
                | event::WRITER_LEASE_CONFLICTED
        );
        if !relevant {
            return Ok(());
        }
        let payload: LeaseEventPayload = from_value(envelope.payload.clone())?;
        let _timing = (
            payload.expires_at_ms,
            payload.ttl_ms,
            payload.renew_after_ms,
        );
        let ours = self.lease.as_ref().is_some_and(|grant| {
            grant.lease.generation == payload.generation
                && grant.lease.holder_id == payload.holder_id
                && grant.lease.controller_instance_id == payload.controller_instance_id
        });
        if envelope.event == event::WRITER_LEASE_REVOKED
            && payload.reason == Some(LeaseReason::SessionEnded)
        {
            self.observe_session_ended();
            return Ok(());
        }
        if self.is_terminal() {
            return Ok(());
        }
        match envelope.event.as_str() {
            event::WRITER_LEASE_REVOKED | event::WRITER_LEASE_EXPIRED if ours => {
                let retry = payload.reason == Some(LeaseReason::Expired);
                self.set_read_only(None, true, retry);
            }
            event::WRITER_LEASE_ACQUIRED if !ours && self.lease.is_none() => {
                self.set_read_only(Some(payload.holder_id), false, self.retry_mode.enabled());
            }
            _ => {}
        }
        Ok(())
    }
}

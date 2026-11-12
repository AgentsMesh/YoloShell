use std::time::Duration;
use tokio::time::Instant;
use yolo_hub_terminal_protocol::LeaseGrant;

pub(crate) struct Authority {
    pub grant: LeaseGrant,
    pub renew_at: Instant,
    pub expires_at: Instant,
}
impl Authority {
    pub fn received(grant: LeaseGrant, sent_at: Instant, now: Instant) -> Option<Self> {
        let expires_at = sent_at + Duration::from_millis(grant.fields().ttl_ms);
        (now < expires_at).then_some(Self {
            grant,
            renew_at: sent_at + Duration::from_millis(grant.fields().renew_after_ms),
            expires_at,
        })
    }
    pub fn is_current(&self, now: Instant) -> bool {
        now < self.expires_at
    }
}

pub(crate) fn retire_expired(authority: &mut Option<Authority>, now: Instant) -> bool {
    if authority
        .as_ref()
        .is_some_and(|value| !value.is_current(now))
    {
        *authority = None;
        true
    } else {
        false
    }
}

use crate::{Attempt, Controller, Failure, Intent};
use std::time::Instant;
use uuid::Uuid;

impl Controller {
    pub fn due(&mut self, now: Instant, connected: bool, has_grant: bool) -> bool {
        self.expire(now);
        connected && !has_grant && self.ready(now)
    }

    /// The caller supplies fresh, non-nil attempt and connection identities.
    pub fn begin(&mut self, request_id: Uuid, connection: Uuid, now: Instant) -> Option<Attempt> {
        self.expire(now);
        if request_id.is_nil() || connection.is_nil() || !self.ready(now) {
            return None;
        }
        let attempt = Attempt {
            target: self.target,
            request_id,
            connection,
        };
        self.schedule(attempt, now);
        Some(attempt)
    }

    /// A valid reply still requires the adapter to validate the remote grant.
    pub fn succeeded(&mut self, attempt: Attempt, now: Instant) -> bool {
        if !self.matches(attempt, now) {
            return false;
        }
        self.adopt(now);
        true
    }

    pub fn failed(&mut self, attempt: Attempt, failure: Failure, now: Instant) -> bool {
        if !self.matches(attempt, now) {
            return false;
        }
        self.record_failure(failure, now);
        true
    }

    fn ready(&self, now: Instant) -> bool {
        self.intent == Intent::Write
            && self.pending.is_none()
            && self.failure != Some(Failure::Denied)
            && self.retry_at.is_none_or(|retry| now >= retry)
    }

    fn matches(&mut self, attempt: Attempt, now: Instant) -> bool {
        self.expire(now);
        self.intent == Intent::Write && self.pending.is_some_and(|(current, _)| current == attempt)
    }
}

use crate::{ATTEMPT_TIMEOUT, Attempt, Failure, Intent, Presentation, RETRY_INTERVAL};
use std::time::Instant;
use yolo_hub_identity::RuntimeTarget;

pub struct Controller {
    pub(crate) target: RuntimeTarget,
    pub(crate) intent: Intent,
    pub(crate) pending: Option<(Attempt, Instant)>,
    pub(crate) retry_at: Option<Instant>,
    pub(crate) failure: Option<Failure>,
    pub(crate) recovering: bool,
}

impl Controller {
    pub fn new(target: RuntimeTarget) -> Self {
        Self {
            target,
            intent: Intent::Observe,
            pending: None,
            retry_at: None,
            failure: None,
            recovering: false,
        }
    }

    pub fn target(&self) -> RuntimeTarget {
        self.target
    }

    pub fn intent(&self) -> Intent {
        self.intent
    }

    pub fn failure(&self) -> Option<Failure> {
        self.failure
    }

    pub fn pending(&self) -> Option<Attempt> {
        self.pending.map(|(attempt, _)| attempt)
    }

    /// Explicit intent survives transient failures; repeating it cannot duplicate
    /// an in-flight acquisition or bypass contention backoff.
    pub fn request_write(&mut self, now: Instant) {
        if self.intent == Intent::Observe || matches!(self.failure, Some(Failure::Denied | Failure::Transient)) {
            self.retry_at = Some(now);
            self.failure = None;
        }
        self.intent = Intent::Write;
    }

    /// Keep the controller as a tombstone for late acquisition completions.
    pub fn observe(&mut self) {
        self.intent = Intent::Observe;
        self.pending = None;
        self.retry_at = None;
        self.failure = None;
        self.recovering = false;
    }

    /// The adapter may adopt an independently validated initial or takeover grant.
    /// Neither this method nor any other policy transition proves remote authority.
    pub fn adopt(&mut self, now: Instant) {
        self.intent = Intent::Write;
        self.pending = None;
        self.retry_at = Some(now);
        self.failure = None;
        self.recovering = true;
    }

    pub fn disconnected(&mut self, now: Instant) {
        self.lost(now);
    }

    pub fn lost(&mut self, now: Instant) {
        self.pending = None;
        if self.intent == Intent::Write && self.failure.is_none() {
            self.record_failure(Failure::Transient, now);
        }
    }

    /// The adapter first validates the exact grant for a renewal failure.
    pub fn record_failure(&mut self, failure: Failure, now: Instant) {
        self.pending = None;
        if self.intent == Intent::Write {
            self.failure = Some(failure);
            self.retry_at = Some(now + RETRY_INTERVAL);
            self.recovering = true;
        }
    }

    pub fn phase(&self, now: Instant, has_grant: bool) -> Presentation {
        if self.intent == Intent::Observe {
            return Presentation::ReadOnly;
        }
        if has_grant {
            return Presentation::Writing;
        }
        match self.failure {
            Some(Failure::Contended) => Presentation::Occupied,
            Some(Failure::Denied) => Presentation::Denied,
            Some(Failure::Transient) => Presentation::Recovering,
            None if self.recovering
                || self.pending.is_some_and(|(_, deadline)| now >= deadline) =>
            {
                Presentation::Recovering
            }
            None => Presentation::Acquiring,
        }
    }

    pub(crate) fn expire(&mut self, now: Instant) {
        if self.pending.is_some_and(|(_, deadline)| now >= deadline) {
            self.pending = None;
            self.failure = Some(Failure::Transient);
            self.retry_at = Some(now + RETRY_INTERVAL);
            self.recovering = true;
        }
    }

    pub(crate) fn schedule(&mut self, attempt: Attempt, now: Instant) {
        self.pending = Some((attempt, now + ATTEMPT_TIMEOUT));
    }
}

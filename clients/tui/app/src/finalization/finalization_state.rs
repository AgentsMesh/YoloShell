use std::time::{Duration, Instant};
use uuid::Uuid;
use yolo_hub_client_channels::{ControlRequest, RequestScope, RequestValidity};
use yolo_hub_rpc_protocol::Command;
use yolo_tui_creation_journal::PendingFinalization;

pub(crate) const RETRY_DELAY: Duration = Duration::from_secs(1);
pub(crate) const SAVE_FAILED_LABEL: &str = "Shell finalization failed";

pub(crate) struct Attempt {
    pub generation: Uuid,
    pub validity: RequestValidity,
    pub scope: RequestScope,
}

pub(crate) struct Finalization {
    pub record: PendingFinalization,
    pub tab: Option<Uuid>,
    pub pane: Option<Uuid>,
    pub attempt: Option<Attempt>,
    pub retry_at: Instant,
    pub rejection: Option<String>,
    pub acknowledged: bool,
}

impl Finalization {
    pub fn command(&self, creation_token: Uuid) -> Command {
        Command::Finalize {
            creation_token,
            creation: self.record.creation,
        }
    }

    pub fn matches_attempt(&self, request: &ControlRequest) -> bool {
        self.attempt.as_ref().is_some_and(|attempt| {
            request.generation == Some(attempt.generation)
                && request.validity.as_ref().is_some_and(|validity| {
                    attempt.validity.same_scope(validity)
                        && attempt.validity.deadline() == validity.deadline()
                })
        })
    }

    pub fn retry(&mut self, now: Instant) {
        self.attempt.take();
        self.retry_at = now + RETRY_DELAY;
    }
}

impl Drop for Attempt {
    fn drop(&mut self) {
        // An older exact success may settle this identity while its replacement
        // attempt is queued. Retire that local scope before it can dispatch.
        self.scope.retire();
    }
}

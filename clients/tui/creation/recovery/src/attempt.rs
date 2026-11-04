use std::time::Instant;
use uuid::Uuid;
use yolo_hub_client_channels::{ControlRequest, RequestScope, RequestValidity};

pub(crate) struct Attempt {
    pub generation: Uuid,
    pub validity: RequestValidity,
    pub scope: RequestScope,
}

impl Attempt {
    pub fn matches(&self, request: &ControlRequest, now: Instant) -> bool {
        request.generation == Some(self.generation)
            && self.validity.deadline() > now
            && self.validity.is_current()
            && request.validity.as_ref().is_some_and(|validity| {
                self.validity.same_scope(validity)
                    && self.validity.deadline() == validity.deadline()
                    && validity.is_current()
            })
    }
}

impl Drop for Attempt {
    fn drop(&mut self) {
        self.scope.retire();
    }
}

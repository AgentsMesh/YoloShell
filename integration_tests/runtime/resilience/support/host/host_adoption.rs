use super::*;

pub(super) const ADOPTION_PENDING_DETAIL: &str = "replacement Host has not adopted runtime";

#[derive(Clone)]
pub(super) struct RuntimeAdoptionExpectation {
    bound: runtime_record_authority::BoundRuntimeRecord,
    observed: std::sync::Arc<std::sync::Mutex<runtime_record_authority::BoundRuntimeRecord>>,
}

impl RuntimeAdoptionExpectation {
    fn new(bound: runtime_record_authority::BoundRuntimeRecord) -> Self {
        Self {
            observed: std::sync::Arc::new(std::sync::Mutex::new(bound.clone())),
            bound,
        }
    }

    fn record(&self) -> &RuntimeRecord {
        self.bound.record()
    }
}

#[derive(Debug, Eq, PartialEq)]
enum AdoptionProgress {
    Complete,
    Pending(String),
}

#[path = "host_adoption_capture.rs"]
mod capture;
#[path = "host_adoption_evaluate.rs"]
mod evaluate;
#[path = "host_adoption_snapshot.rs"]
mod snapshot;
#[path = "host_adoption_wait.rs"]
mod wait;

#[cfg(test)]
use evaluate::{evaluate_runtime, evaluate_runtime_with_observation};
pub(super) use wait::wait_for_runtime_adoptions;
#[cfg(test)]
use wait::{ADOPTION_TIMEOUT_DETAIL, pending_session_view_error, wait_for_adoption_progress};

#[cfg(test)]
#[path = "host_adoption_evaluation_tests.rs"]
mod evaluation_tests;
#[cfg(test)]
#[path = "host_adoption_tests.rs"]
mod tests;

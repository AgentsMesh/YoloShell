use super::protocol::BrokerRequest;
use std::sync::Mutex;
use uuid::Uuid;

pub type BrokerAdoptionAcknowledgement = Box<dyn FnOnce(Uuid) -> bool + Send + 'static>;

pub(super) fn permits_pending_request(request: &BrokerRequest) -> bool {
    matches!(
        request,
        BrokerRequest::Status | BrokerRequest::Bootstrap | BrokerRequest::PrepareHubTransport(_)
    )
}

pub(super) struct BrokerAdmission {
    state: Mutex<AdmissionState>,
}

#[derive(Clone, Copy, Eq, PartialEq)]
enum AdmissionState {
    PendingAdoption,
    PreparingAdoption,
    Open,
    Closed,
}

impl BrokerAdmission {
    pub(super) fn new(pending_adoption: bool) -> Self {
        Self {
            state: Mutex::new(if pending_adoption {
                AdmissionState::PendingAdoption
            } else {
                AdmissionState::Open
            }),
        }
    }

    pub(super) fn is_open(&self) -> bool {
        *self.state.lock().unwrap() != AdmissionState::Closed
    }

    pub(super) fn is_adopted(&self) -> bool {
        *self.state.lock().unwrap() == AdmissionState::Open
    }

    pub(super) fn adopt(&self, instance_id: Uuid, acknowledge: BrokerAdoptionAcknowledgement) {
        {
            let mut state = self.state.lock().unwrap();
            if *state != AdmissionState::PendingAdoption {
                return;
            }
            *state = AdmissionState::PreparingAdoption;
        }
        let accepted = match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            acknowledge(instance_id)
        })) {
            Ok(accepted) => accepted,
            Err(payload) => {
                dispose_adoption_panic(payload);
                false
            }
        };
        let mut state = self.state.lock().unwrap();
        if *state == AdmissionState::PreparingAdoption {
            *state = if accepted {
                AdmissionState::Open
            } else {
                AdmissionState::Closed
            };
        }
    }

    pub(super) fn promote(&self, promotion: impl FnOnce()) -> bool {
        let state = self.state.lock().unwrap();
        if *state != AdmissionState::Open {
            return false;
        }
        promotion();
        true
    }

    pub(super) fn close(&self) {
        *self.state.lock().unwrap() = AdmissionState::Closed;
    }
}

fn dispose_adoption_panic(payload: Box<dyn std::any::Any + Send>) {
    if let Err(secondary) = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| drop(payload)))
    {
        std::mem::forget(secondary);
    }
}

#[cfg(test)]
#[cfg(any(not(test), feature = "yolo-unit-tests"))]
#[path = "../../unit_tests/server/server_admission_tests.rs"]
mod tests;

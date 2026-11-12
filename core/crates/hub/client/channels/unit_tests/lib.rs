use crate::{ControlError, admission::admit};
use std::error::Error;
use uuid::Uuid;
use yolo_hub_client::ClientError;
#[path = "../test_support/commands.rs"]
mod commands;
#[path = "../test_support/constants.rs"]
mod constants;
mod input_completion;
mod input_stream;
mod lease_authority;
mod platform_events;
mod request_validity;

#[test]
fn disconnected_and_stale_requests_are_distinct_before_any_io() {
    let current = Uuid::new_v4();
    let previous = Uuid::new_v4();
    for requested in [None, Some(current), Some(previous)] {
        assert!(matches!(
            admit(None, requested),
            Err(ControlError::NotConnected)
        ));
    }
    for requested in [None, Some(previous), Some(Uuid::nil())] {
        assert!(matches!(
            admit(Some(current), requested),
            Err(ControlError::StaleGeneration)
        ));
    }
    assert!(admit(Some(current), Some(current)).is_ok());
}

#[test]
fn reconnect_generation_cannot_admit_an_old_request() {
    let previous = Uuid::new_v4();
    let next = Uuid::new_v4();
    assert!(admit(Some(previous), Some(previous)).is_ok());
    assert!(matches!(
        admit(None, Some(previous)),
        Err(ControlError::NotConnected)
    ));
    assert!(matches!(
        admit(Some(next), Some(previous)),
        Err(ControlError::StaleGeneration)
    ));
    assert!(admit(Some(next), Some(next)).is_ok());
}

#[test]
fn admission_errors_have_no_hidden_client_cause() {
    for error in [
        ControlError::NotConnected,
        ControlError::StaleGeneration,
        ControlError::OperationLimit,
        ControlError::QueryLimit,
        ControlError::QueryTaskFailed,
    ] {
        assert!(error.source().is_none());
        assert!(!error.to_string().is_empty());
    }
    let error = ControlError::from(ClientError::Poisoned);
    assert!(error.source().is_some());
    assert_eq!(error.to_string(), ClientError::Poisoned.to_string());
}

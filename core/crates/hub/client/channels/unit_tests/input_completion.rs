use super::commands::{grant, input};
use crate::{
    ControlError, ControlRequest, RequestScope,
    input_completion::{Dispatch, settle},
};
use std::time::Instant;
use uuid::Uuid;
use yolo_hub_client::ClientError;
use yolo_hub_identity::HubId;
use yolo_hub_rpc_protocol::{Command, Reply};
use yolo_hub_terminal_protocol::PublicError;

fn request(scope: &RequestScope) -> ControlRequest {
    let hub = HubId::new();
    ControlRequest {
        generation: Some(Uuid::new_v4()),
        destination: hub,
        command: input(grant(hub)),
        validity: Some(scope.validity_until(Instant::now() + super::constants::TIMEOUT)),
    }
}

#[test]
fn rejected_unconfirmed_and_illegal_input_results_cancel_dependents_before_publication() {
    for result in [
        Err(ControlError::Invalidated),
        Err(ControlError::NotConnected),
        Err(ControlError::StaleGeneration),
        Err(ControlError::Client(ClientError::Remote(
            PublicError::OutcomeUnknown,
        ))),
        Err(ControlError::Client(ClientError::OutcomeUnknownResponse(
            PublicError::InvalidPayload,
        ))),
        Ok(Reply::Completed {}),
    ] {
        let scope = RequestScope::ordered_inputs();
        let prefix = request(&scope);
        let suffix = request(&scope);
        settle(&prefix, &result, Dispatch::Attempted);
        assert!(!suffix.validity.as_ref().unwrap().is_current());
    }
}

#[test]
fn acknowledged_input_including_duplicate_acknowledgement_preserves_dependents() {
    for duplicate in [false, true] {
        let scope = RequestScope::ordered_inputs();
        let prefix = request(&scope);
        let suffix = request(&scope);
        settle(
            &prefix,
            &Ok(Reply::Input { duplicate }),
            Dispatch::Attempted,
        );
        assert!(suffix.validity.as_ref().unwrap().is_current());
    }
}

#[test]
fn ordinary_requests_and_non_input_results_do_not_gain_ordered_failure_semantics() {
    let ordinary = RequestScope::default();
    let request = request(&ordinary);
    settle(
        &request,
        &Err(ControlError::Invalidated),
        Dispatch::NotAttempted,
    );
    assert!(request.validity.as_ref().unwrap().is_current());
    let ordered = RequestScope::ordered_inputs();
    let mut other = request.clone();
    other.validity = Some(ordered.validity_until(Instant::now() + super::constants::TIMEOUT));
    other.command = Command::Hello {};
    settle(
        &other,
        &Err(ControlError::Invalidated),
        Dispatch::NotAttempted,
    );
    assert!(other.validity.as_ref().unwrap().is_current());
    let mut unscoped = request;
    unscoped.validity = None;
    settle(
        &unscoped,
        &Err(ControlError::Invalidated),
        Dispatch::NotAttempted,
    );
}

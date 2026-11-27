use super::{creation_support::Fixture, finalization_support::*};
use crate::{events, network::Update};
use yolo_hub_client::ClientError;
use yolo_hub_client_channels::ControlError;
use yolo_hub_rpc_protocol::Reply;
use yolo_hub_terminal_protocol::PublicError;

#[test]
fn transient_finalize_failures_retry_only_the_same_receipt_until_completed() {
    for error in [
        ControlError::OperationLimit,
        ControlError::NotConnected,
        ControlError::StaleGeneration,
        ControlError::Invalidated,
        ControlError::Client(ClientError::Poisoned),
        ControlError::Client(ClientError::Remote(PublicError::Internal)),
        ControlError::Client(ClientError::Remote(PublicError::Unreachable)),
        ControlError::Client(ClientError::Remote(PublicError::OutcomeUnknown)),
        ControlError::Client(ClientError::OutcomeUnknownResponse(
            PublicError::InvalidPayload,
        )),
    ] {
        let mut fixture = Fixture::new();
        let (_, created, first) = confirmed(&mut fixture);
        finish(&mut fixture, first.clone(), Err(error));
        assert_eq!(fixture.state.creations.finalizing.len(), 1);
        assert_eq!(fixture.original_binding(), Some(created.address));
        assert!(!first.validity.as_ref().unwrap().is_current());
        let retry = retry(&mut fixture, &first);
        assert_eq!(retry.command, first.command);
        assert_eq!(retry.generation, first.generation);
        assert!(
            !retry
                .validity
                .as_ref()
                .unwrap()
                .same_scope(first.validity.as_ref().unwrap())
        );
        finish(&mut fixture, retry, Ok(Reply::Completed {}));
        assert!(fixture.state.creations.finalizing.is_empty());
        assert!(fixture.state.writers.is_empty());
        no_commands(&mut fixture);
    }
}

#[test]
fn asynchronous_queue_rejection_retries_with_the_current_connection_after_reconnect() {
    let mut fixture = Fixture::new();
    let (_, _, first) = confirmed(&mut fixture);
    events::apply(
        &mut fixture.state,
        &fixture.jobs,
        Update::NotSent {
            request: first.clone(),
        },
    );
    fixture.state.disconnected();
    crate::finalization_actions::flush(&mut fixture.state, &fixture.jobs, later());
    no_commands(&mut fixture);
    let fresh = uuid::Uuid::new_v4();
    events::apply(
        &mut fixture.state,
        &fixture.jobs,
        Update::Control(crate::client_capability_support::ready(fresh)),
    );
    let retry = retry(&mut fixture, &first);
    assert_eq!(retry.generation, Some(fresh));
    assert_eq!(retry.command, first.command);
    finish(&mut fixture, retry, Ok(Reply::Completed {}));
    assert!(fixture.state.creations.finalizing.is_empty());
    no_commands(&mut fixture);
}

#[test]
fn permanent_finalize_rejection_retains_evidence_without_automatic_retry() {
    for result in [
        Err(ControlError::Client(ClientError::Remote(
            PublicError::Unauthorized,
        ))),
        Err(ControlError::Client(ClientError::Remote(
            PublicError::Conflict,
        ))),
        Err(ControlError::Client(ClientError::Unsupported)),
        Ok(Reply::Input { duplicate: false }),
    ] {
        let mut fixture = Fixture::new();
        let (_, _, request) = confirmed(&mut fixture);
        finish(&mut fixture, request.clone(), result);
        let entry = &fixture.state.creations.finalizing[&token(&request)];
        assert!(entry.rejection.is_some());
        assert!(entry.attempt.is_none());
        assert!(
            fixture
                .state
                .status
                .starts_with(crate::finalization_state::SAVE_FAILED_LABEL)
        );
        crate::finalization_actions::flush(&mut fixture.state, &fixture.jobs, later());
        no_commands(&mut fixture);
        assert_eq!(fixture.state.creations.finalizing.len(), 1);
    }
}

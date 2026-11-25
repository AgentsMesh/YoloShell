use super::query_support::*;
use crate::{events, network::Update};
use yolo_hub_client::ClientError;
use yolo_hub_client_channels::ControlError;
use yolo_hub_rpc_protocol::Reply;
use yolo_hub_terminal_protocol::PublicError;

#[test]
fn success_local_invalidation_and_queue_rejection_retire_only_the_matching_pending_reply() {
    for outcome in [
        ResultKind::Success,
        ResultKind::Invalidated,
        ResultKind::NotSent,
    ] {
        let mut fixture = Fixture::new();
        fixture.emit(DSR);
        let request = fixture.queued();
        let id = input(&request).fields().request_id;
        match outcome {
            ResultKind::Success => {
                fixture.finish(request, Ok(Reply::Input { duplicate: false }), true)
            }
            ResultKind::Invalidated => {
                fixture.finish(request, Err(ControlError::Invalidated), true)
            }
            ResultKind::NotSent => events::apply(
                &mut fixture.state,
                &fixture.jobs,
                Update::NotSent { request },
            ),
        }
        assert!(!fixture.state.queries.pending.contains_key(&id));
        assert_eq!(
            fixture.state.writers[&fixture.target.address].grant,
            fixture.grant
        );
        fixture.emit(POSITION);
        fixture.no_jobs();
    }
}
enum ResultKind {
    Success,
    Invalidated,
    NotSent,
}

#[test]
fn uncertain_or_invalid_query_result_retires_exact_writer_without_replaying_input() {
    for result in [
        Err(ControlError::Client(ClientError::OutcomeUnknownResponse(
            PublicError::InvalidPayload,
        ))),
        Err(ControlError::Client(ClientError::Remote(
            PublicError::OutcomeUnknown,
        ))),
        Ok(Reply::Completed {}),
    ] {
        let mut fixture = Fixture::new();
        fixture.emit(DSR);
        let request = fixture.queued();
        let validity = request.validity.clone().unwrap();
        fixture.finish(request, result, true);
        assert!(!fixture.state.writers.contains_key(&fixture.target.address));
        assert!(!validity.is_current());
        assert!(fixture.state.queries.pending.is_empty());
        fixture.emit(DSR);
        fixture.no_jobs();
    }
}

#[test]
fn late_unknown_or_duplicate_reply_cannot_drop_replacement_writer() {
    for replace_connection in [false, true] {
        let mut fixture = Fixture::new();
        fixture.emit(DSR);
        let request = fixture.queued();
        if replace_connection {
            fixture.state.disconnected();
            fixture.state.control_generation = Some(uuid::Uuid::new_v4());
            crate::client_capability_support::install(&mut fixture.state);
        }
        let replacement = fixture.replace_writer();
        fixture.finish(
            request.clone(),
            Err(ControlError::Client(ClientError::OutcomeUnknownResponse(
                PublicError::InvalidPayload,
            ))),
            !replace_connection,
        );
        assert_eq!(
            fixture.state.writers[&fixture.target.address].grant,
            replacement
        );
        fixture.finish(request, Ok(Reply::Input { duplicate: false }), true);
        assert_eq!(
            fixture.state.writers[&fixture.target.address].grant,
            replacement
        );
        fixture.no_jobs();
    }
}

#[test]
fn a_tampered_completion_cannot_consume_a_different_pending_input() {
    let mut fixture = Fixture::new();
    fixture.emit(DSR);
    let original = fixture.queued();
    let mut changed = original.clone();
    let mut fields = input(&changed).fields().clone();
    fields.data = ATTRIBUTE_REPLY.to_vec();
    changed.command = yolo_hub_rpc_protocol::Command::Input {
        request: yolo_hub_terminal_protocol::InputRequest::new(fields).unwrap(),
    };
    fixture.finish(changed, Ok(Reply::Input { duplicate: false }), true);
    assert_eq!(
        fixture
            .state
            .queries
            .pending
            .get(&input(&original).fields().request_id)
            .map(|pending| &pending.request),
        Some(input(&original))
    );
    fixture.finish(original, Ok(Reply::Input { duplicate: false }), true);
    assert!(fixture.state.queries.pending.is_empty());
}

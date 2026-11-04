use super::creation_support::*;
use crate::{
    creation_actions, events,
    network::{Job, Update},
};
use yolo_hub_client::ClientError;
use yolo_hub_client_channels::{ControlError, ControlEvent};
use yolo_hub_terminal_protocol::{CreateShellRequest, PublicError};

#[test]
fn queue_failure_or_duplicate_pending_id_never_registers_or_replays_creation() {
    let mut fixture = Fixture::with_capacity(SINGLE_JOB);
    let queued = fixture.start();
    assert!(!creation_actions::enqueue(
        &mut fixture.state,
        &fixture.jobs,
        fixture.origin,
        create(&queued).clone()
    ));
    assert!(fixture.incoming.try_recv().is_err());
    assert!(
        fixture
            .jobs
            .try_send(Job::Refresh {
                request_id: uuid::Uuid::new_v4()
            })
            .is_ok()
    );
    let mut fields = create(&queued).fields().clone();
    fields.request_id = uuid::Uuid::new_v4();
    fields.creation_token = uuid::Uuid::new_v4();
    let next = CreateShellRequest::new(fields).unwrap();
    assert!(!creation_actions::enqueue(
        &mut fixture.state,
        &fixture.jobs,
        fixture.origin,
        next.clone()
    ));
    assert!(
        !fixture
            .state
            .creations
            .pending
            .contains_key(&next.fields().request_id)
    );
    assert_eq!(fixture.state.status, crate::constants::QUEUE_FULL_LABEL);
}

#[test]
fn failed_unsent_or_unknown_creation_cannot_later_bind_any_view() {
    for retired in [Retired::NotSent, Retired::Failure, Retired::UnknownFailure] {
        let mut fixture = Fixture::new();
        let request = fixture.start();
        match retired {
            Retired::NotSent => events::apply(
                &mut fixture.state,
                &fixture.jobs,
                Update::NotSent {
                    request: request.clone(),
                },
            ),
            Retired::Failure => events::apply(
                &mut fixture.state,
                &fixture.jobs,
                Update::Control(ControlEvent::Finished {
                    request: request.clone(),
                    result: Err(ControlError::Client(ClientError::Remote(
                        PublicError::Conflict,
                    ))),
                    connected: true,
                }),
            ),
            Retired::UnknownFailure => events::apply(
                &mut fixture.state,
                &fixture.jobs,
                Update::Control(ControlEvent::Finished {
                    request: request.clone(),
                    result: Err(ControlError::Client(ClientError::OutcomeUnknownResponse(
                        PublicError::InvalidPayload,
                    ))),
                    connected: false,
                }),
            ),
        }
        assert!(fixture.state.creations.pending.is_empty());
        let before = fixture.state.tab_layout().clone();
        let created = fixture.created();
        fixture.finish(request.clone(), created);
        assert_eq!(*fixture.state.tab_layout(), before);
        fixture.assert_finalize(&request, created, false);
    }
}
enum Retired {
    NotSent,
    Failure,
    UnknownFailure,
}

#[test]
fn mismatched_request_origin_and_control_generation_never_consume_binding_authority() {
    for mismatch in [Mismatch::Request, Mismatch::Origin, Mismatch::Generation] {
        let mut fixture = Fixture::new();
        let request = fixture.start();
        let mut supplied = create(&request).clone();
        let mut created = fixture.created();
        match mismatch {
            Mismatch::Request => {
                let mut fields = supplied.fields().clone();
                fields.creation_token = uuid::Uuid::new_v4();
                supplied = CreateShellRequest::new(fields).unwrap();
            }
            Mismatch::Origin => created.address.hub_id = yolo_hub_identity::HubId::new(),
            Mismatch::Generation => fixture.state.control_generation = Some(uuid::Uuid::new_v4()),
        }
        crate::creation_events::created(&mut fixture.state, &fixture.jobs, supplied, created);
        assert_eq!(fixture.original_binding(), Some(fixture.expected));
        assert!(fixture.incoming.try_recv().is_err());
        assert!(
            fixture
                .state
                .creations
                .pending
                .contains_key(&create(&request).fields().request_id)
        );
    }
}
enum Mismatch {
    Request,
    Origin,
    Generation,
}

#[test]
fn finalization_enqueue_failure_retains_receipt_and_retries_only_finalize() {
    let mut fixture = Fixture::with_capacity(SINGLE_JOB);
    let request = fixture.start();
    assert!(
        fixture
            .jobs
            .try_send(Job::Refresh {
                request_id: uuid::Uuid::new_v4()
            })
            .is_ok()
    );
    let created = fixture.created();
    crate::creation_events::created(
        &mut fixture.state,
        &fixture.jobs,
        create(&request).clone(),
        created,
    );
    assert!(fixture.state.creations.pending.is_empty());
    assert_eq!(fixture.state.creations.finalizing.len(), SINGLE_JOB);
    assert!(matches!(
        fixture.incoming.try_recv().unwrap(),
        Job::Refresh { .. }
    ));
    crate::creation_events::created(
        &mut fixture.state,
        &fixture.jobs,
        create(&request).clone(),
        created,
    );
    assert!(fixture.incoming.try_recv().is_err());
    let token = create(&request).fields().creation_token;
    let retry_at = fixture.state.creations.finalizing[&token].retry_at;
    crate::finalization_actions::flush(&mut fixture.state, &fixture.jobs, retry_at);
    fixture.assert_finalize(&request, created, true);
    assert_eq!(fixture.state.creations.finalizing.len(), SINGLE_JOB);
}

#[test]
fn slow_unanswered_creations_have_a_fixed_registration_limit() {
    let mut fixture = Fixture::new();
    let first = fixture.start();
    let limit = crate::creation_constants::MAX_PENDING_CREATIONS;
    for _ in SINGLE_JOB..limit {
        fixture.start();
    }
    let mut fields = create(&first).fields().clone();
    fields.request_id = uuid::Uuid::new_v4();
    fields.creation_token = uuid::Uuid::new_v4();
    assert!(!creation_actions::enqueue(
        &mut fixture.state,
        &fixture.jobs,
        fixture.origin,
        CreateShellRequest::new(fields).unwrap()
    ));
    assert_eq!(fixture.state.creations.pending.len(), limit);
    assert_eq!(fixture.original_binding(), Some(fixture.expected));
    assert!(fixture.incoming.try_recv().is_err());
}

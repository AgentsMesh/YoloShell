use super::{
    creation_reconnect::{assert_finalized, reconnect},
    creation_support::*,
};
use crate::{events, network::Update};
use uuid::Uuid;
use yolo_hub_client::ClientError;
use yolo_hub_client_channels::{ControlError, ControlEvent, ControlRequest};
use yolo_hub_identity::HubId;
use yolo_hub_rpc_protocol::{Command, Reply};
use yolo_hub_terminal_protocol::{CreateShellRequest, PublicError};

#[test]
fn old_generation_failure_or_unsent_retires_only_its_exact_pending_creation() {
    for unsent in [false, true] {
        let mut fixture = Fixture::new();
        let admitted = fixture.start();
        reconnect(&mut fixture);
        let other = fixture.start();
        let generation = fixture.state.control_generation;
        fail(&mut fixture, admitted.clone(), unsent);
        assert!(
            !fixture
                .state
                .creations
                .pending
                .contains_key(&create(&admitted).fields().request_id)
        );
        assert!(
            fixture
                .state
                .creations
                .pending
                .contains_key(&create(&other).fields().request_id)
        );
        assert_eq!(fixture.state.control_generation, generation);
        assert!(fixture.state.writers.is_empty());
        let created = fixture.created();
        fixture.finish(admitted.clone(), created);
        assert_eq!(fixture.original_binding(), Some(fixture.expected));
        assert_finalized(&mut fixture, &admitted, created, false);
        fixture.finish(other.clone(), created);
        assert_eq!(fixture.original_binding(), Some(created.address));
        assert_finalized(&mut fixture, &other, created, true);
    }
}

#[test]
fn unrelated_completion_or_unsent_cannot_consume_an_admitted_creation() {
    for mismatch in [
        Mismatch::Generation,
        Mismatch::MissingGeneration,
        Mismatch::Origin,
        Mismatch::Request,
    ] {
        for outcome in [Outcome::Success, Outcome::Failure, Outcome::NotSent] {
            let mut fixture = Fixture::new();
            let admitted = fixture.start();
            reconnect(&mut fixture);
            let generation = fixture.state.control_generation;
            let mut unrelated = admitted.clone();
            match mismatch {
                Mismatch::Generation => unrelated.generation = generation,
                Mismatch::MissingGeneration => unrelated.generation = None,
                Mismatch::Origin => unrelated.destination = HubId::new(),
                Mismatch::Request => {
                    let mut fields = create(&unrelated).fields().clone();
                    fields.creation_token = Uuid::new_v4();
                    unrelated.command = Command::Create {
                        request: CreateShellRequest::new(fields).unwrap(),
                    };
                }
            }
            let created = fixture.created();
            match outcome {
                Outcome::Success => fixture.finish(unrelated, created),
                Outcome::Failure => fail(&mut fixture, unrelated, false),
                Outcome::NotSent => fail(&mut fixture, unrelated, true),
            }
            assert!(
                fixture
                    .state
                    .creations
                    .pending
                    .contains_key(&create(&admitted).fields().request_id)
            );
            assert_eq!(fixture.original_binding(), Some(fixture.expected));
            assert_eq!(fixture.state.control_generation, generation);
            assert!(fixture.state.writers.is_empty());
            assert_finalized(&mut fixture, &admitted, created, false);
            fixture.finish(admitted.clone(), created);
            assert_eq!(fixture.original_binding(), Some(created.address));
            assert_finalized(&mut fixture, &admitted, created, true);
        }
    }
}

#[test]
fn non_creation_callbacks_cannot_consume_creation_intent() {
    let mut fixture = Fixture::new();
    let admitted = fixture.start();
    let mut unrelated = admitted.clone();
    unrelated.command = Command::Hello {};
    assert!(!crate::creation_completion::matches(
        &fixture.state,
        &unrelated
    ));
    crate::creation_completion::apply(
        &mut fixture.state,
        &fixture.jobs,
        &unrelated,
        Ok(Reply::Completed {}),
    );
    assert!(
        fixture
            .state
            .creations
            .pending
            .contains_key(&create(&admitted).fields().request_id)
    );
    assert_eq!(fixture.original_binding(), Some(fixture.expected));
    assert!(fixture.incoming.try_recv().is_err());
}

#[test]
fn invalid_creation_reply_retires_intent_without_binding_or_finalization() {
    let mut fixture = Fixture::new();
    let admitted = fixture.start();
    crate::creation_completion::apply(
        &mut fixture.state,
        &fixture.jobs,
        &admitted,
        Ok(Reply::Completed {}),
    );
    assert!(fixture.state.creations.pending.is_empty());
    assert_eq!(fixture.original_binding(), Some(fixture.expected));
    assert_eq!(
        fixture.state.status,
        crate::constants::UNEXPECTED_REPLY_LABEL
    );
    assert!(fixture.incoming.try_recv().is_err());
}

#[derive(Clone, Copy)]
enum Mismatch {
    Generation,
    MissingGeneration,
    Origin,
    Request,
}
enum Outcome {
    Success,
    Failure,
    NotSent,
}

fn fail(fixture: &mut Fixture, request: ControlRequest, unsent: bool) {
    let update = if unsent {
        Update::NotSent { request }
    } else {
        Update::Control(ControlEvent::Finished {
            request,
            result: Err(ControlError::Client(ClientError::Remote(
                PublicError::Conflict,
            ))),
            connected: true,
        })
    };
    events::apply(&mut fixture.state, &fixture.jobs, update);
}

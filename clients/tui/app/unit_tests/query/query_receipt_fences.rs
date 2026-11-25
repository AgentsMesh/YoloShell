use super::query_support::*;
use std::time::Instant;
use yolo_hub_client::ClientError;
use yolo_hub_client_channels::{ControlError, ControlRequest, RequestScope};
use yolo_hub_rpc_protocol::{Command, Reply};
use yolo_hub_terminal_protocol::{InputRequest, LeaseGrant, PublicError};

fn lost_response() -> Result<Reply, ControlError> {
    Err(ControlError::Client(ClientError::OutcomeUnknownResponse(
        PublicError::InvalidPayload,
    )))
}

#[test]
fn missing_or_altered_query_receipt_authority_cannot_manufacture_current_connection_loss() {
    let tamper: &[fn(&mut ControlRequest)] = &[
        |request| request.validity = None,
        |request| request.generation = Some(uuid::Uuid::new_v4()),
        |request| request.destination = yolo_hub_identity::HubId::new(),
        |request| {
            request.validity = Some(
                RequestScope::default()
                    .validity_until(request.validity.as_ref().unwrap().deadline()),
            );
        },
        |request| {
            let mut fields = input(request).fields().clone();
            fields.data = ATTRIBUTE_REPLY.to_vec();
            request.command = Command::Input {
                request: InputRequest::new(fields).unwrap(),
            };
        },
    ];
    for tamper in tamper {
        let mut fixture = Fixture::new();
        fixture.emit(DSR);
        let original = fixture.queued();
        let mut changed = original.clone();
        tamper(&mut changed);
        let generation = fixture.state.control_generation;
        fixture.finish(changed, lost_response(), false);
        assert_eq!(fixture.state.control_generation, generation);
        assert_eq!(
            fixture.state.writers[&fixture.target.address].grant,
            fixture.grant
        );
        assert!(
            fixture.state.queries.pending[&input(&original).fields().request_id].matches(&original)
        );
        assert!(original.validity.as_ref().unwrap().is_current());
        fixture.finish(original, Ok(Reply::Input { duplicate: false }), true);
        assert!(fixture.state.queries.pending.is_empty());
        fixture.no_jobs();
    }
}

#[test]
fn a_same_query_scope_with_a_different_deadline_is_not_the_original_receipt() {
    let mut fixture = Fixture::new();
    fixture.emit(DSR);
    let original = fixture.queued();
    let mut fields = *fixture.grant.fields();
    fields.ttl_ms += LEASE_EXTENSION_MS;
    let renewed = LeaseGrant::new(fields).unwrap();
    fixture.state.record_grant(renewed, Instant::now());
    fixture.emit(DSR);
    let newer = fixture.queued();
    assert!(
        original
            .validity
            .as_ref()
            .unwrap()
            .same_scope(newer.validity.as_ref().unwrap())
    );
    assert!(
        original.validity.as_ref().unwrap().deadline()
            < newer.validity.as_ref().unwrap().deadline()
    );
    let mut changed = original.clone();
    changed.validity = newer.validity.clone();
    let generation = fixture.state.control_generation;
    fixture.finish(changed, lost_response(), false);
    assert_eq!(fixture.state.control_generation, generation);
    assert_eq!(
        fixture.state.writers[&fixture.target.address].grant,
        renewed
    );
    assert!(
        fixture.state.queries.pending[&input(&original).fields().request_id].matches(&original)
    );
    fixture.finish(original, Ok(Reply::Input { duplicate: false }), true);
    fixture.finish(newer, Ok(Reply::Input { duplicate: false }), true);
    assert!(fixture.state.queries.pending.is_empty());
}

#[test]
fn duplicate_and_unknown_query_receipts_cannot_revoke_the_current_writer() {
    for unknown in [false, true] {
        for missing_validity in [false, true] {
            let mut fixture = Fixture::new();
            fixture.emit(DSR);
            let mut old = fixture.queued();
            fixture.finish(old.clone(), Ok(Reply::Input { duplicate: false }), true);
            if unknown {
                let mut fields = input(&old).fields().clone();
                fields.request_id = uuid::Uuid::new_v4();
                old.command = Command::Input {
                    request: InputRequest::new(fields).unwrap(),
                };
            }
            if missing_validity {
                old.validity = None;
            }
            let generation = fixture.state.control_generation;
            fixture.finish(old, lost_response(), false);
            assert_eq!(fixture.state.control_generation, generation);
            assert_eq!(
                fixture.state.writers[&fixture.target.address].grant,
                fixture.grant
            );
            fixture.no_jobs();
        }
    }
}

#[test]
fn a_correlated_query_failure_keeps_its_real_current_connection_loss_semantics() {
    let mut fixture = Fixture::new();
    fixture.emit(DSR);
    let request = fixture.queued();
    let validity = request.validity.clone().unwrap();
    fixture.finish(request, lost_response(), false);
    assert!(fixture.state.control_generation.is_none());
    assert!(fixture.state.writers.is_empty());
    assert!(fixture.state.queries.pending.is_empty());
    assert!(!validity.is_current());
    fixture.emit(DSR);
    fixture.no_jobs();
}

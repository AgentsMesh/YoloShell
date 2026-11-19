use super::query_support::{Fixture, STEP};
use crate::{Mode, network::Job, terminal_actions};
use std::time::Instant;
use yolo_hub_client::ClientError;
use yolo_hub_client_channels::{ControlError, ControlRequest};
use yolo_hub_rpc_protocol::{Command, Reply};
use yolo_hub_terminal_protocol::PublicError;

const INPUT: &[u8] = b"exact original input";
const CHANGED_INPUT: &[u8] = b"different input";
const NEXT_TAB: &str = "Next tab";

fn queued(fixture: &mut Fixture) -> ControlRequest {
    fixture.state.mode = Mode::Terminal;
    assert!(terminal_actions::deliver(
        &mut fixture.state,
        &fixture.jobs,
        Ok(INPUT.to_vec())
    ));
    let Job::UserInput(request) = fixture.incoming.try_recv().unwrap() else {
        panic!()
    };
    request
}

const SINGLE_JOB: usize = 1;

#[test]
fn every_local_authority_fence_invalidates_queued_input_before_transport_dispatch() {
    let retire: &[fn(&mut Fixture)] = &[
        |f| {
            f.state.retire_writer(f.target.address);
        },
        |f| {
            f.replace_writer();
        },
        |f| {
            f.state.disconnected();
        },
        |f| {
            f.state.control_generation = Some(uuid::Uuid::new_v4());
        },
        |f| {
            f.state.writers.get_mut(&f.target.address).unwrap().expires = Instant::now();
        },
        |f| {
            f.state
                .tab_layout_mut()
                .split(
                    yolo_presentation_layout::Axis::Horizontal,
                    Some(f.target.address),
                )
                .unwrap();
        },
        |f| {
            f.state.bind_focused(None).unwrap();
        },
        |f| {
            f.state.tabs.create(NEXT_TAB).unwrap();
        },
        |f| {
            f.state.mode = Mode::Inventory;
        },
        |f| {
            f.state.mark_resync(f.target.address);
        },
        |f| {
            f.state.replace_terminals(vec![]);
        },
    ];
    for retire in retire {
        let mut fixture = Fixture::new();
        let request = queued(&mut fixture);
        assert!(request.validity.as_ref().unwrap().is_current());
        retire(&mut fixture);
        fixture.state.sync_query_authority();
        assert!(!request.validity.as_ref().unwrap().is_current());
        fixture.finish(request, Err(ControlError::Invalidated), true);
        assert!(fixture.state.inputs.pending.is_empty());
        fixture.no_jobs();
    }
}

#[test]
fn confirmed_renewal_extends_queued_authority_without_changing_input_receipt_identity() {
    let mut fixture = Fixture::new();
    let request = queued(&mut fixture);
    let original_deadline = request.validity.as_ref().unwrap().deadline();
    let mut fields = *fixture.grant.fields();
    fields.expires_at_ms += super::query_support::LEASE_EXTENSION_MS as i64;
    fields.ttl_ms += super::query_support::LEASE_EXTENSION_MS;
    let renewal = yolo_hub_terminal_protocol::LeaseGrant::new(fields).unwrap();
    fixture.state.record_grant(renewal, Instant::now());
    assert!(request.validity.as_ref().unwrap().is_current());
    assert_eq!(
        request.validity.as_ref().unwrap().deadline(),
        original_deadline
    );
    assert!(request.validity.as_ref().unwrap().admission_deadline() > original_deadline);
    fixture.finish(request, Ok(Reply::Input { duplicate: false }), true);
    assert!(fixture.state.inputs.pending.is_empty());
    assert_eq!(
        fixture.state.writers[&fixture.target.address].grant,
        renewal
    );
}

#[test]
fn failed_or_uncertain_input_retires_only_its_exact_writer_and_never_replays() {
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
        let request = queued(&mut fixture);
        fixture.finish(request, result, true);
        assert!(!fixture.state.writers.contains_key(&fixture.target.address));
        assert!(fixture.state.inputs.pending.is_empty());
        fixture.no_jobs();
    }
}

#[test]
fn a_tampered_completion_cannot_retire_another_input_or_its_writer() {
    let mut fixture = Fixture::new();
    let original = queued(&mut fixture);
    let mut changed = original.clone();
    let Command::Input { request } = changed.command else {
        panic!()
    };
    let mut fields = request.fields().clone();
    fields.data = CHANGED_INPUT.to_vec();
    changed.command = Command::Input {
        request: yolo_hub_terminal_protocol::InputRequest::new(fields).unwrap(),
    };
    fixture.finish(changed, Ok(Reply::Input { duplicate: false }), true);
    assert!(!fixture.state.inputs.pending.is_empty());
    assert!(fixture.state.writers.contains_key(&fixture.target.address));
    fixture.finish(original, Ok(Reply::Input { duplicate: false }), true);
    assert!(fixture.state.inputs.pending.is_empty());
}

#[test]
fn late_unknown_input_cannot_drop_a_replacement_writer() {
    let mut fixture = Fixture::new();
    let request = queued(&mut fixture);
    crate::terminal_input_authority::request_resume(&mut fixture.state, fixture.target.address);
    let replacement = fixture.replace_writer();
    fixture.finish(
        request,
        Err(ControlError::Client(ClientError::Remote(
            PublicError::OutcomeUnknown,
        ))),
        true,
    );
    assert_eq!(
        fixture.state.writers[&fixture.target.address].grant,
        replacement
    );
    fixture.no_jobs();
}

#[test]
fn same_writer_renewal_keeps_deferred_bytes_and_refreshes_the_scope_deadline() {
    let mut fixture = Fixture::with_capacity(SINGLE_JOB);
    fixture.state.mode = Mode::Terminal;
    fixture
        .jobs
        .try_send(Job::Refresh {
            request_id: uuid::Uuid::new_v4(),
        })
        .unwrap();
    assert!(terminal_actions::deliver(
        &mut fixture.state,
        &fixture.jobs,
        Ok(b"prefix".to_vec())
    ));
    assert!(terminal_actions::deliver(
        &mut fixture.state,
        &fixture.jobs,
        Ok(b"suffix".to_vec())
    ));
    let mut fields = *fixture.grant.fields();
    fields.ttl_ms += STEP;
    fields.expires_at_ms += i64::try_from(STEP).unwrap();
    let renewed = yolo_hub_terminal_protocol::LeaseGrant::new(fields).unwrap();
    fixture
        .state
        .record_grant(renewed, std::time::Instant::now());
    assert_eq!(fixture.state.inputs.deferred.len(), 1);
    assert!(fixture.state.inputs.deferred[0].current(&fixture.state));
}

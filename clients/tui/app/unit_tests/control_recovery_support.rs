use super::query_support::{Fixture, STEP, snapshot};
use crate::{
    Mode, actions, events,
    network::{Job, ObservationUpdate, Update},
};
use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers};
use std::time::Instant;
use yolo_hub_client_channels::ControlRequest;
use yolo_hub_rpc_protocol::Command;
use yolo_hub_terminal_protocol::{
    AttachRequest, AttachResult, AttachResultFields, FIRST_OUTPUT_SEQUENCE, LeaseGrant, LeaseMode,
    SecretCapability,
};
use yolo_tui_terminal_control::RETRY_INTERVAL;

pub(super) const NEW_INPUT: &[u8] = b"\r";
pub(super) const OCCUPIED_LABEL: &str = "Input control occupied";
pub(super) const DENIED_LABEL: &str = "Input access denied";
pub(super) const RESTORING_LABEL: &str = "Restoring input";

pub(super) fn elapsed_retry(fixture: &mut Fixture) {
    fixture
        .state
        .control
        .disconnected(Instant::now() - RETRY_INTERVAL);
}

pub(super) fn acquisition(fixture: &mut Fixture) -> ControlRequest {
    let Job::Acquire {
        generation,
        origin,
        request,
        validity,
    } = fixture.incoming.try_recv().unwrap()
    else {
        panic!("expected a scoped acquisition")
    };
    assert_eq!(request.mode, LeaseMode::Acquire {});
    assert_eq!(request.target, fixture.target);
    ControlRequest {
        generation: Some(generation),
        destination: origin,
        command: Command::Lease { request },
        validity: Some(validity),
    }
}

pub(super) fn replacement(fixture: &Fixture) -> LeaseGrant {
    let mut fields = *fixture.grant.fields();
    fields.generation += STEP;
    fields.capability = SecretCapability::generate();
    LeaseGrant::new(fields).unwrap()
}

pub(super) fn reconnect_output(fixture: &mut Fixture) {
    let cursor = fixture.state.surfaces[&fixture.target.address].cursor();
    let attachment = AttachResult::new(AttachResultFields {
        target: fixture.target,
        snapshot: Some(snapshot(fixture.target, cursor.after_sequence, &[])),
        chunks: Vec::new(),
        history: None,
        next_sequence: cursor.after_sequence + FIRST_OUTPUT_SEQUENCE,
        resynchronized: true,
    })
    .unwrap();
    fixture.scoped(ObservationUpdate::Attached {
        request: AttachRequest::new(fixture.target, Some(cursor)).unwrap(),
        attachment,
    });
}

pub(super) fn release_key(fixture: &mut Fixture) {
    fixture.state.mode = Mode::Terminal;
    for key in [
        KeyEvent::new(KeyCode::Char('b'), KeyModifiers::CONTROL),
        KeyEvent::new(KeyCode::Char('r'), KeyModifiers::NONE),
    ] {
        assert!(!actions::handle(
            &mut fixture.state,
            &fixture.jobs,
            fixture.target.address.hub_id,
            Event::Key(key)
        ));
    }
}

pub(super) fn ready(fixture: &mut Fixture) {
    events::apply(
        &mut fixture.state,
        &fixture.jobs,
        Update::Control(crate::client_capability_support::ready(uuid::Uuid::new_v4())),
    );
    assert!(matches!(
        fixture.incoming.try_recv().unwrap(),
        Job::Refresh { .. }
    ));
}

pub(super) fn released(fixture: &mut Fixture, expected: LeaseGrant) {
    assert!(matches!(fixture.incoming.try_recv().unwrap(), Job::Call {
        command: Command::Release { grant }, ..
    } if grant == expected));
}

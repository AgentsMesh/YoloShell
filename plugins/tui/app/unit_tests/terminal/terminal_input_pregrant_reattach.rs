use super::query_support::{Fixture, STEP};
use crate::{
    Mode, StartupAction, events,
    network::{Job, Update},
    polling, terminal_actions,
};
use crossterm::event::{
    Event, KeyCode, KeyEvent, KeyModifiers, MouseButton, MouseEvent, MouseEventKind,
};
use std::time::Instant;
use yolo_hub_client_channels::{ControlError, ControlEvent, ControlRequest};
use yolo_hub_identity::{TerminalAddress, TerminalId};
use yolo_hub_rpc_protocol::{Command, Reply};
use yolo_hub_terminal_protocol::{
    CreateShellResult, LeaseGrant, RuntimeStatus, SecretCapability, TerminalSummary,
};

const TYPED: &[u8] = b"echo hi\r";

fn reattaching() -> (Fixture, ControlRequest) {
    let mut fixture = Fixture::new();
    fixture.state.mode = Mode::Terminal;
    fixture.state.retire_writer(fixture.target.address);
    polling::acquire(&mut fixture.state, &fixture.jobs, false);
    let Job::Acquire {
        generation,
        origin,
        request,
        validity,
    } = fixture.incoming.try_recv().unwrap()
    else {
        panic!("a reattach must acquire writer authority");
    };
    let request = ControlRequest {
        generation: Some(generation),
        destination: origin,
        command: Command::Lease { request },
        validity: Some(validity),
    };
    (fixture, request)
}

fn typed(fixture: &mut Fixture) {
    assert!(terminal_actions::deliver(
        &mut fixture.state,
        &fixture.jobs,
        Ok(TYPED.to_vec()),
    ));
}

fn only_input(fixture: &mut Fixture) -> Vec<u8> {
    let Job::UserInput(input) = fixture.incoming.try_recv().unwrap() else {
        panic!("pregrant input must use the input queue");
    };
    let Command::Input { request } = input.command else {
        panic!("pregrant input must use the input command");
    };
    request.fields().data.clone()
}

#[test]
fn typing_during_reattach_acquisition_is_buffered_and_delivered_on_grant() {
    let (mut fixture, request) = reattaching();
    typed(&mut fixture);
    assert_eq!(fixture.state.inputs.pregrant.len(), 1);
    assert_eq!(fixture.state.status, crate::constants::INPUT_WAITING_LABEL);
    assert!(fixture.incoming.try_recv().is_err());

    fixture.finish(
        request,
        Ok(Reply::Lease {
            grant: fixture.grant,
        }),
        true,
    );

    assert!(fixture.state.inputs.pregrant.is_empty());
    assert_eq!(fixture.state.status, crate::constants::CONNECTED_LABEL);
    assert_eq!(only_input(&mut fixture), TYPED);
    assert!(fixture.incoming.try_recv().is_err());
}

#[test]
fn typing_prompts_without_a_snapshot_are_encoded_for_the_pending_grant() {
    let (mut fixture, request) = reattaching();
    fixture.state.surfaces.remove(&fixture.target.address);
    crate::actions::handle(
        &mut fixture.state,
        &fixture.jobs,
        fixture.target.address.hub_id,
        Event::Key(KeyEvent::new(KeyCode::Char('x'), KeyModifiers::NONE)),
    );
    assert_eq!(fixture.state.inputs.pregrant.len(), 1);
    assert_eq!(fixture.state.status, crate::constants::INPUT_WAITING_LABEL);

    fixture.finish(
        request,
        Ok(Reply::Lease {
            grant: fixture.grant,
        }),
        true,
    );
    assert_eq!(only_input(&mut fixture), b"x");
}

#[test]
fn bytes_after_the_grant_but_before_the_first_snapshot_are_sent() {
    let mut fixture = Fixture::new();
    fixture.state.mode = Mode::Terminal;
    fixture.state.surfaces.remove(&fixture.target.address);
    assert!(terminal_actions::deliver(
        &mut fixture.state,
        &fixture.jobs,
        Ok(TYPED.to_vec()),
    ));
    assert!(
        fixture
            .state
            .inputs
            .snapshot_pending
            .contains(&fixture.target.address)
    );
    assert_eq!(only_input(&mut fixture), TYPED);
}

#[test]
fn bytes_during_a_geometry_resync_are_queued_until_ready() {
    let mut fixture = Fixture::new();
    fixture.state.mode = Mode::Terminal;
    fixture.state.surfaces.remove(&fixture.target.address);
    fixture.state.mark_resync(fixture.target.address);
    assert!(terminal_actions::deliver(
        &mut fixture.state,
        &fixture.jobs,
        Ok(TYPED.to_vec()),
    ));
    assert_eq!(fixture.state.status, crate::constants::INPUT_WAITING_LABEL);
    assert_eq!(fixture.state.inputs.pregrant.len(), 1);
    fixture.no_jobs();

    fixture.state.clear_resync(fixture.target.address);
    crate::terminal_input::drain_pregrant(&mut fixture.state, &fixture.jobs);
    assert!(fixture.state.inputs.pregrant.is_empty());
    assert_eq!(only_input(&mut fixture), TYPED);
}

#[test]
fn a_successful_receipt_resolves_an_interrupted_prefix_when_nothing_remains() {
    let mut fixture = Fixture::new();
    let request = super::terminal_input_ordering::queued(&mut fixture, b"x");
    let address = fixture.target.address;
    fixture
        .state
        .inputs
        .interrupted
        .insert(address, fixture.grant);
    fixture.state.status = crate::constants::INPUT_WAITING_LABEL.into();
    fixture.finish(request, Ok(Reply::Input { duplicate: false }), true);
    assert!(!fixture.state.inputs.interrupted.contains_key(&address));
    assert!(!crate::terminal_input_authority::blocked(
        &fixture.state,
        address
    ));
    assert_eq!(fixture.state.status, crate::constants::CONNECTED_LABEL);
}

#[test]
fn a_failed_receipt_keeps_the_interruption_even_if_a_later_input_succeeds() {
    let mut fixture = Fixture::new();
    let first = super::terminal_input_ordering::queued(&mut fixture, b"x");
    let second = super::terminal_input_ordering::queued(&mut fixture, b"y");
    let address = fixture.target.address;
    fixture
        .state
        .inputs
        .interrupted
        .insert(address, fixture.grant);
    fixture.finish(first, Err(ControlError::Invalidated), true);
    assert!(fixture.state.inputs.failures.contains(&address));
    fixture.finish(second, Ok(Reply::Input { duplicate: false }), true);
    assert!(fixture.state.inputs.interrupted.contains_key(&address));
    assert!(crate::terminal_input_authority::blocked(
        &fixture.state,
        address
    ));
}


include!("pregrant_discard_resume.rs");
include!("pregrant_vacant_shell.rs");
include!("pregrant_new_tab.rs");
include!("pregrant_startup.rs");
include!("pregrant_pointer_blockage.rs");

use super::{
    query_support::Fixture,
    resize_input_support::{RESIZED_WIDTH, resize_request, resized_size, viewport},
    terminal_input_ordering::queued,
};
use crate::{Mode, network::Job, pointer_input, terminal_actions, terminal_input};
use crossterm::event::{KeyModifiers, MouseButton, MouseEvent, MouseEventKind};
use uuid::Uuid;
use yolo_hub_client_channels::ControlRequest;
use yolo_hub_rpc_protocol::Command;

const SINGLE_JOB: usize = 1;
const PREFIX: &[u8] = b"resize-pressure-prefix";
const DEFERRED_INPUT: &[u8] = b"\r";
const VIEW_COORDINATE: u16 = 1;

fn current(request: &ControlRequest) -> bool {
    request
        .validity
        .as_ref()
        .is_some_and(|validity| validity.is_current())
}

fn resize(fixture: &mut Fixture) {
    crate::resize::apply(
        &mut fixture.state,
        &fixture.jobs,
        viewport(fixture.target.address, RESIZED_WIDTH),
    );
}

fn resize_not_sent(fixture: &mut Fixture, request: yolo_hub_terminal_protocol::ResizeRequest) {
    let generation = fixture.state.control_generation;
    crate::channel_events::not_sent(
        &mut fixture.state,
        ControlRequest {
            generation,
            destination: fixture.target.address.hub_id,
            command: Command::Resize { request },
            validity: None,
        },
    );
}

#[test]
fn full_jobs_keep_input_fifo_and_resize_intent_without_revoking_writer() {
    let mut fixture = Fixture::with_capacity(SINGLE_JOB);
    let address = fixture.target.address;
    fixture
        .jobs
        .try_send(Job::Refresh {
            request_id: Uuid::new_v4(),
        })
        .unwrap();
    fixture.state.mode = Mode::Terminal;
    assert!(terminal_actions::deliver(
        &mut fixture.state,
        &fixture.jobs,
        Ok(PREFIX.to_vec()),
    ));
    let first = fixture.state.inputs.waiting.clone().expect("waiting input");
    assert!(terminal_actions::deliver(
        &mut fixture.state,
        &fixture.jobs,
        Ok(DEFERRED_INPUT.to_vec()),
    ));
    assert_eq!(fixture.state.inputs.deferred.len(), 1);
    assert!(fixture.state.inputs.waiting());
    resize(&mut fixture);
    assert!(fixture.state.is_resize_pending(address));
    assert!(!fixture.state.resize_in_flight(address).is_some());
    fixture.state.sync_query_authority();
    assert!(current(&first));
    assert_eq!(fixture.state.writers[&address].grant, fixture.grant);

    assert!(matches!(
        fixture.incoming.try_recv().unwrap(),
        Job::Refresh { .. }
    ));
    terminal_input::flush(&mut fixture.state, fixture.jobs.try_reserve().unwrap());
    let Job::UserInput(dispatched) = fixture.incoming.try_recv().unwrap() else {
        panic!("expected FIFO input")
    };
    assert_eq!(
        super::query_support::input(&dispatched).fields().data,
        PREFIX
    );
    terminal_input::flush(&mut fixture.state, fixture.jobs.try_reserve().unwrap());
    let Job::UserInput(deferred) = fixture.incoming.try_recv().unwrap() else {
        panic!("expected deferred input")
    };
    assert_eq!(
        super::query_support::input(&deferred).fields().data,
        DEFERRED_INPUT
    );
    resize(&mut fixture);
    let resize = resize_request(&mut fixture);
    assert_eq!(resize.size, resized_size(RESIZED_WIDTH));
    assert_eq!(fixture.state.writers[&address].grant, fixture.grant);
    assert_eq!(
        super::query_support::input(&dispatched),
        super::query_support::input(&first)
    );
    fixture.finish(
        dispatched,
        Ok(yolo_hub_rpc_protocol::Reply::Input { duplicate: false }),
        true,
    );
    fixture.finish(
        deferred,
        Ok(yolo_hub_rpc_protocol::Reply::Input { duplicate: false }),
        true,
    );
    fixture.no_jobs();
}

#[test]
fn geometry_sync_blocks_pointer_and_query_replies_but_preserves_keyboard_writer() {
    let mut fixture = Fixture::new();
    let address = fixture.target.address;
    let pending = queued(&mut fixture, PREFIX);
    resize(&mut fixture);
    let _ = resize_request(&mut fixture);
    let view = viewport(address, RESIZED_WIDTH);
    let mouse = MouseEvent {
        kind: MouseEventKind::Down(MouseButton::Left),
        column: VIEW_COORDINATE,
        row: VIEW_COORDINATE,
        modifiers: KeyModifiers::NONE,
    };
    assert!(!pointer_input::forward(
        &mut fixture.state,
        &fixture.jobs,
        view,
        mouse
    ));
    fixture.emit(super::query_support::DSR);
    fixture.no_jobs();
    fixture.state.sync_query_authority();
    assert!(current(&pending));
    assert_eq!(fixture.state.writers[&address].grant, fixture.grant);
}

#[test]
fn not_sent_resize_retries_without_losing_writer_and_replacement_clears_geometry() {
    let mut fixture = Fixture::new();
    let address = fixture.target.address;
    resize(&mut fixture);
    let request = resize_request(&mut fixture);
    resize_not_sent(&mut fixture, request);
    assert!(fixture.state.is_resize_pending(address));
    assert!(fixture.state.is_geometry_syncing(address));
    assert_eq!(fixture.state.writers[&address].grant, fixture.grant);

    resize(&mut fixture);
    let retry = resize_request(&mut fixture);
    assert_eq!(retry.size, resized_size(RESIZED_WIDTH));
    assert_ne!(request.request_id, retry.request_id);
    let replacement = fixture.replace_writer();
    assert!(!fixture.state.is_geometry_syncing(address));
    assert!(!fixture.state.is_resize_pending(address));
    assert_eq!(fixture.state.writers[&address].grant, replacement);

    resize(&mut fixture);
    let replacement_resize = resize_request(&mut fixture);
    assert_ne!(retry.request_id, replacement_resize.request_id);
    fixture.state.disconnected();
    assert!(!fixture.state.is_geometry_syncing(address));
    assert!(!fixture.state.is_resize_pending(address));
    assert!(!fixture.state.resize_in_flight(address).is_some());
    fixture.no_jobs();
}

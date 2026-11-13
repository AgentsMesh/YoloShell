use super::{
    query_support::Fixture,
    resize_input_support::{
        INITIAL_WIDTH, RESIZED_WIDTH, SECOND_RESIZED_WIDTH, attachment, resize_request,
        resized_size, viewport,
    },
    terminal_input_ordering::queued,
};
use crate::{events, network::Update};
use std::time::Instant;
use tempfile::tempdir;
use tokio::sync::mpsc;
use yolo_hub_client::HubEndpoint;
use yolo_hub_client_channels::ControlRequest;
use yolo_hub_rpc_protocol::{Command, Reply};

const INPUT_PREFIX: &[u8] = b"echo resize-prefix";
const INPUT_SUFFIX: &[u8] = b"\r";
const STALE_OUTPUT: &[u8] = b"stale old-size output";
const OBSERVER_SOCKET: &str = "hub.sock";
const WIDTH_STEP: u16 = 1;

fn assert_current(request: &ControlRequest) {
    assert!(
        request
            .validity
            .as_ref()
            .is_some_and(|value| value.is_current())
    );
}

fn resize_completed(fixture: &mut Fixture, request: yolo_hub_terminal_protocol::ResizeRequest) {
    events::apply(
        &mut fixture.state,
        &fixture.jobs,
        Update::Reply {
            command: Command::Resize { request },
            reply: Reply::Completed {},
        },
    );
}

fn attach_snapshot(fixture: &mut Fixture, size: yolo_hub_terminal_protocol::TerminalSize) {
    let generation = fixture
        .state
        .observer_generation(fixture.target.address)
        .expect("observer generation");
    let (request, attachment) = attachment(fixture.target, size);
    events::apply(
        &mut fixture.state,
        &fixture.jobs,
        Update::Observation {
            target: fixture.target,
            generation,
            update: crate::network::ObservationUpdate::Attached {
                request,
                attachment,
            },
        },
    );
}

#[tokio::test]
async fn resize_keeps_unconfirmed_input_and_writer_through_snapshot_recovery() {
    let mut fixture = Fixture::new();
    let pending = queued(&mut fixture, INPUT_PREFIX);
    let address = fixture.target.address;
    let original_grant = fixture.grant;
    let prior_size = fixture.state.surfaces[&address].screen().size();

    crate::resize::apply(
        &mut fixture.state,
        &fixture.jobs,
        viewport(address, RESIZED_WIDTH),
    );
    let request = resize_request(&mut fixture);
    fixture.state.sync_query_authority();
    assert_current(&pending);
    assert_eq!(fixture.state.writers[&address].grant, original_grant);
    assert!(!fixture.state.inputs.interrupted.contains_key(&address));
    let during_resize = queued(&mut fixture, INPUT_SUFFIX);
    assert_current(&during_resize);

    resize_completed(&mut fixture, request);
    assert!(fixture.state.is_resync(address));
    assert_current(&pending);
    assert_eq!(fixture.state.writers[&address].grant, original_grant);
    let cursor = fixture.state.surfaces[&address].cursor();
    fixture.emit(STALE_OUTPUT);
    assert_eq!(fixture.state.surfaces[&address].cursor(), cursor);

    let directory = tempdir().expect("observer socket directory");
    let socket = directory.path().join(OBSERVER_SOCKET);
    let _listener = tokio::net::UnixListener::bind(&socket).expect("observer socket");
    let (updates, _responses) = mpsc::channel(crate::constants::QUEUE_CAPACITY);
    let options = crate::Options {
        endpoint: HubEndpoint::local(socket),
        hub: fixture.target.address.hub_id,
        layout: None,
    };
    let mut observers = crate::observers::Observers::default();
    observers.sync(&options, &mut fixture.state, &updates);
    assert!(!fixture.state.is_resync(address));
    assert_eq!(fixture.state.surfaces[&address].screen().size(), prior_size,);
    let during_attach = queued(&mut fixture, INPUT_SUFFIX);
    assert_current(&during_attach);

    attach_snapshot(&mut fixture, resized_size(RESIZED_WIDTH));
    assert!(!fixture.state.is_geometry_syncing(address));
    assert!(fixture.state.input_ready(address));
    assert_current(&pending);
    fixture.finish(pending, Ok(Reply::Input { duplicate: false }), true);
    fixture.finish(during_resize, Ok(Reply::Input { duplicate: false }), true);
    fixture.finish(during_attach, Ok(Reply::Input { duplicate: false }), true);

    let next = queued(&mut fixture, INPUT_SUFFIX);
    assert_eq!(
        super::query_support::input(&next).fields().grant,
        original_grant
    );
    fixture.no_jobs();
    observers.close().await;
}

#[test]
fn repeated_resize_keeps_one_request_and_latest_geometry_without_takeover() {
    let mut fixture = Fixture::new();
    let pending = queued(&mut fixture, INPUT_PREFIX);
    let address = fixture.target.address;

    crate::resize::apply(
        &mut fixture.state,
        &fixture.jobs,
        viewport(address, RESIZED_WIDTH),
    );
    let first = resize_request(&mut fixture);
    crate::resize::apply(
        &mut fixture.state,
        &fixture.jobs,
        viewport(address, SECOND_RESIZED_WIDTH),
    );
    assert_eq!(
        fixture.state.last_size(address).expect("last size").cols(),
        SECOND_RESIZED_WIDTH
    );
    fixture.state.sync_query_authority();
    assert_current(&pending);
    assert_eq!(fixture.state.writers[&address].grant, fixture.grant);

    resize_completed(&mut fixture, first);
    assert!(fixture.state.is_resize_pending(address));
    assert!(!fixture.state.has_resync());
    crate::resize::apply(
        &mut fixture.state,
        &fixture.jobs,
        viewport(address, SECOND_RESIZED_WIDTH),
    );
    let second = resize_request(&mut fixture);
    assert_ne!(first.request_id, second.request_id);
    assert_eq!(second.size, resized_size(SECOND_RESIZED_WIDTH));
    resize_completed(&mut fixture, second);
    assert!(fixture.state.is_resync(address));
    assert_current(&pending);
    assert!(fixture.state.writers.contains_key(&address));
    fixture.no_jobs();
}

#[test]
fn same_writer_renewal_during_resize_keeps_pending_input_authority() {
    let mut fixture = Fixture::new();
    let pending = queued(&mut fixture, INPUT_PREFIX);
    let address = fixture.target.address;
    crate::resize::apply(
        &mut fixture.state,
        &fixture.jobs,
        viewport(address, INITIAL_WIDTH + WIDTH_STEP),
    );
    let resize = resize_request(&mut fixture);
    let mut fields = *fixture.grant.fields();
    fields.ttl_ms += super::fixtures::TTL_MS;
    fields.expires_at_ms += super::fixtures::TTL_MS as i64;
    let renewed = yolo_hub_terminal_protocol::LeaseGrant::new(fields).expect("renewed grant");
    fixture.state.record_grant(renewed, Instant::now());

    assert_current(&pending);
    assert_eq!(fixture.state.writers[&address].grant, renewed);
    resize_completed(&mut fixture, resize);
    assert_eq!(fixture.state.writers[&address].grant, renewed);
    assert_current(&pending);
    fixture.no_jobs();
}

use super::{
    fixtures,
    resize_input_support::{
        RESIZED_WIDTH, attachment, resize_request, resized_size, unreachable, viewport,
    },
    terminal_input_ordering::queued,
};
use crate::{
    events,
    network::{ObservationUpdate, Update},
};
use std::time::Instant;
use yolo_hub_client::ClientError;
use yolo_hub_client_channels::{ControlError, ControlRequest};
use yolo_hub_rpc_protocol::{Command, Reply};
use yolo_hub_terminal_protocol::TerminalEvent;

const INPUT_PREFIX: &[u8] = b"echo resize-failure";
const INPUT_SUFFIX: &[u8] = b"\r";
const FAILURE_MESSAGE: &str = "resize unavailable";

fn input_current(request: &ControlRequest) -> bool {
    request
        .validity
        .as_ref()
        .is_some_and(|validity| validity.is_current())
}

fn resize_failed(fixture: &mut super::query_support::Fixture) {
    let request = resize_request(fixture);
    events::apply(
        &mut fixture.state,
        &fixture.jobs,
        Update::Failed {
            command: Some(Command::Resize { request }),
            message: FAILURE_MESSAGE.into(),
            connected: true,
            transient: false,
        },
    );
}

#[test]
fn resize_failure_keeps_writer_and_retries_latest_geometry() {
    let mut fixture = super::query_support::Fixture::new();
    let pending = queued(&mut fixture, INPUT_PREFIX);
    let address = fixture.target.address;
    crate::resize::apply(
        &mut fixture.state,
        &fixture.jobs,
        viewport(address, RESIZED_WIDTH),
    );
    resize_failed(&mut fixture);

    assert!(input_current(&pending));
    assert_eq!(fixture.state.writers[&address].grant, fixture.grant);
    fixture.state.sync_query_authority();
    let next = queued(&mut fixture, INPUT_SUFFIX);
    assert_eq!(
        super::query_support::input(&next).fields().grant,
        fixture.grant
    );

    crate::resize::apply(
        &mut fixture.state,
        &fixture.jobs,
        viewport(address, RESIZED_WIDTH),
    );
    let retry = resize_request(&mut fixture);
    assert_eq!(retry.size, resized_size(RESIZED_WIDTH));
    fixture.finish(pending, Ok(Reply::Input { duplicate: false }), true);
    fixture.finish(next, Ok(Reply::Input { duplicate: false }), true);
    fixture.no_jobs();
}

#[test]
fn output_gap_and_offline_still_fence_unconfirmed_input_during_resize() {
    let mut gap = super::query_support::Fixture::new();
    let gap_input = queued(&mut gap, INPUT_PREFIX);
    let gap_address = gap.target.address;
    crate::resize::apply(
        &mut gap.state,
        &gap.jobs,
        viewport(gap_address, RESIZED_WIDTH),
    );
    let _ = resize_request(&mut gap);
    gap.scoped(ObservationUpdate::Terminal {
        event: TerminalEvent::ResyncRequired { target: gap.target },
    });
    gap.state.sync_query_authority();
    assert!(!input_current(&gap_input));
    assert!(!gap.state.writers.contains_key(&gap_address));
    gap.no_jobs();

    let mut offline = super::query_support::Fixture::new();
    let offline_input = queued(&mut offline, INPUT_PREFIX);
    let offline_address = offline.target.address;
    crate::resize::apply(
        &mut offline.state,
        &offline.jobs,
        viewport(offline_address, RESIZED_WIDTH),
    );
    let _ = resize_request(&mut offline);
    offline.scoped(ObservationUpdate::Terminal {
        event: unreachable(offline.target),
    });
    offline.state.sync_query_authority();
    assert!(!input_current(&offline_input));
    assert!(!offline.state.writers.contains_key(&offline_address));
    offline.no_jobs();
}

#[test]
fn unknown_input_receipt_during_resize_requires_a_new_writer() {
    let mut fixture = super::query_support::Fixture::new();
    let pending = queued(&mut fixture, INPUT_PREFIX);
    let address = fixture.target.address;
    crate::resize::apply(
        &mut fixture.state,
        &fixture.jobs,
        viewport(address, RESIZED_WIDTH),
    );
    let _ = resize_request(&mut fixture);
    fixture.finish(
        pending,
        Err(ControlError::Client(ClientError::Remote(
            yolo_hub_terminal_protocol::PublicError::OutcomeUnknown,
        ))),
        true,
    );
    assert!(!fixture.state.writers.contains_key(&address));
    assert!(crate::terminal_input_authority::blocked(
        &fixture.state,
        address,
    ));
    fixture.no_jobs();
}

#[test]
fn snapshot_attach_after_resize_does_not_change_renewed_writer() {
    let mut fixture = super::query_support::Fixture::new();
    let pending = queued(&mut fixture, INPUT_PREFIX);
    let address = fixture.target.address;
    crate::resize::apply(
        &mut fixture.state,
        &fixture.jobs,
        viewport(address, RESIZED_WIDTH),
    );
    let request = resize_request(&mut fixture);
    let mut fields = *fixture.grant.fields();
    fields.ttl_ms += fixtures::TTL_MS;
    fields.expires_at_ms += fixtures::TTL_MS as i64;
    let renewed = yolo_hub_terminal_protocol::LeaseGrant::new(fields).expect("renewed grant");
    fixture.state.record_grant(renewed, Instant::now());
    events::apply(
        &mut fixture.state,
        &fixture.jobs,
        Update::Reply {
            command: Command::Resize { request },
            reply: Reply::Completed {},
        },
    );
    assert_eq!(fixture.state.writers[&address].grant, renewed);

    let generation = uuid::Uuid::new_v4();
    fixture.state.clear_resync(address);
    fixture.state.set_observer_generation(address, generation);
    let (attach_request, attachment) = attachment(fixture.target, resized_size(RESIZED_WIDTH));
    events::apply(
        &mut fixture.state,
        &fixture.jobs,
        Update::Observation {
            target: fixture.target,
            generation,
            update: ObservationUpdate::Attached {
                request: attach_request,
                attachment,
            },
        },
    );
    assert_eq!(fixture.state.writers[&address].grant, renewed);
    assert!(input_current(&pending));
    fixture.no_jobs();
}

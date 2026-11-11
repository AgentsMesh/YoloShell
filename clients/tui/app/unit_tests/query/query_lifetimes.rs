use super::query_support::*;
use crate::{
    events,
    network::{ObservationUpdate, Update},
};
use std::time::Instant;
use yolo_hub_client_channels::ControlEvent;
use yolo_hub_terminal_protocol::{LeaseGrant, TerminalEvent};

#[test]
fn partial_query_cannot_cross_writer_release_replacement_expiry_or_connection_epochs() {
    for transition in [
        Transition::Writer,
        Transition::Release,
        Transition::ExpiredRenewal,
        Transition::Observer,
        Transition::Control,
        Transition::Resync,
    ] {
        let mut fixture = Fixture::new();
        fixture.emit(PARTIAL_QUERY);
        fixture.no_jobs();
        match transition {
            Transition::Writer => {
                fixture.replace_writer();
            }
            Transition::Release => {
                fixture.state.retire_writer(fixture.target.address);
                fixture.state.record_grant(fixture.grant, Instant::now());
                fixture.ready_size();
            }
            Transition::ExpiredRenewal => {
                fixture
                    .state
                    .writers
                    .get_mut(&fixture.target.address)
                    .unwrap()
                    .expires = Instant::now();
                fixture.state.record_grant(fixture.grant, Instant::now());
                fixture.ready_size();
            }
            Transition::Observer => {
                fixture.observer = uuid::Uuid::new_v4();
                fixture
                    .state
                    .set_observer_generation(fixture.target.address, fixture.observer);
            }
            Transition::Control => {
                fixture.state.disconnected();
                fixture.state.control_generation = Some(uuid::Uuid::new_v4());
                crate::client_capability_support::install(&mut fixture.state);
                fixture.state.record_grant(fixture.grant, Instant::now());
                fixture.ready_size();
            }
            Transition::Resync => {
                fixture.state.mark_resync(fixture.target.address);
                fixture.state.sync_query_authority();
                fixture.state.clear_resync(fixture.target.address);
            }
        }
        fixture.emit(QUERY_FINAL);
        fixture.no_jobs();
        fixture.emit(CPR);
        assert_eq!(input(&fixture.queued()).fields().data, CURSOR_REPLY);
    }
}

enum Transition {
    Writer,
    Release,
    ExpiredRenewal,
    Observer,
    Control,
    Resync,
}

#[test]
fn same_writer_renewal_preserves_partial_query_and_existing_queue_deadline() {
    let mut fixture = Fixture::new();
    fixture.emit(DSR);
    let queued = fixture.queued();
    let original_deadline = queued.validity.as_ref().unwrap().deadline();
    fixture.emit(PARTIAL_QUERY);
    let mut fields = *fixture.grant.fields();
    fields.ttl_ms += LEASE_EXTENSION_MS;
    fields.expires_at_ms += LEASE_EXTENSION_MS as i64;
    let renewed = LeaseGrant::new(fields).unwrap();
    crate::lease_events::granted(&mut fixture.state, &fixture.jobs, renewed);
    fixture.emit(QUERY_FINAL);
    let reply = fixture.queued();
    assert_eq!(input(&reply).fields().data, CURSOR_REPLY);
    assert_eq!(input(&reply).fields().grant, renewed);
    assert!(queued.validity.as_ref().unwrap().is_current());
    assert_eq!(queued.validity.unwrap().deadline(), original_deadline);
}

#[test]
fn scoped_terminal_end_or_disconnect_invalidates_all_queued_replies() {
    for unavailable in [false, true] {
        let mut fixture = Fixture::new();
        fixture.emit(DSR);
        let request = fixture.queued();
        let event = if unavailable {
            TerminalEvent::Unreachable {
                target: fixture.target,
            }
        } else {
            TerminalEvent::Exited {
                cursor: fixture.state.surfaces[&fixture.target.address].cursor(),
                code: None,
                signal: None,
            }
        };
        fixture.scoped(ObservationUpdate::Terminal { event });
        assert!(!request.validity.unwrap().is_current());
        assert!(fixture.state.queries.pending.is_empty());
        assert_eq!(
            fixture.state.writers.contains_key(&fixture.target.address),
            unavailable
        );
        fixture.no_jobs();
    }
    let mut fixture = Fixture::new();
    fixture.emit(DSR);
    let request = fixture.queued();
    let generation = fixture.state.control_generation;
    events::apply(
        &mut fixture.state,
        &fixture.jobs,
        Update::Control(ControlEvent::Unreachable {
            generation,
            message: STALE_MESSAGE.into(),
        }),
    );
    assert!(!request.validity.unwrap().is_current());
    fixture.no_jobs();
}

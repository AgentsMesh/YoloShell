use super::{drawing_support, fixtures};
use crate::{State, catalog_events, constants::*, inventory_actions, network::Job};
use crossterm::event::KeyCode;
use std::time::Instant;
use tokio::sync::mpsc;
use uuid::Uuid;
use yolo_hub_catalog_client::{Catalog, CatalogError};
use yolo_hub_client_channels::CatalogEvent;

const REFRESH_KEY: char = 'r';
const WIDTH: u16 = 160;
const HEIGHT: u16 = 12;
const ONE_JOB: usize = 1;

fn refreshing(state: &State) -> bool {
    drawing_support::render(state, WIDTH, HEIGHT)
        .0
        .contains(CATALOG_REFRESHING_LABEL)
}

#[test]
fn explicit_refresh_waits_for_its_new_catalogue_without_revoking_the_existing_writer() {
    let session = fixtures::session();
    let grant = fixtures::grant(&session);
    let address = session.fields().address;
    let generation = Uuid::new_v4();
    let mut state = crate::client_capability_support::state();
    state.control_generation = Some(generation);
    state.replace_terminals(vec![session.clone()]);
    state.inventory.current = true;
    state.record_grant(grant, Instant::now());
    let (jobs, mut incoming) = mpsc::channel(QUEUE_CAPACITY);
    catalog_events::refresh(&mut state, &jobs);
    let previous = state.catalog_pending.unwrap();
    incoming.try_recv().unwrap();
    assert!(!refreshing(&state));

    assert!(!inventory_actions::handle(
        &mut state,
        &jobs,
        KeyCode::Char(REFRESH_KEY),
    ));
    let requested = state.catalog_pending.unwrap();
    assert_ne!(requested, previous);
    assert!(
        matches!(incoming.try_recv().unwrap(), Job::Refresh { request_id } if request_id == requested)
    );
    assert!(refreshing(&state));
    let catalog = Catalog {
        terminals: vec![session],
        ..Catalog::default()
    };
    for result in [Ok(catalog.clone()), Err(CatalogError::Limit)] {
        catalog_events::apply(
            &mut state,
            CatalogEvent {
                request_id: previous,
                result,
            },
        );
        assert_eq!(state.catalog_pending, Some(requested));
        assert!(!state.inventory.current);
        assert!(refreshing(&state));
    }
    assert_eq!(state.control_generation, Some(generation));
    assert_eq!(state.grant(address, Instant::now()), Some(grant));
    catalog_events::apply(
        &mut state,
        CatalogEvent {
            request_id: requested,
            result: Ok(catalog),
        },
    );
    assert!(state.catalog_pending.is_none());
    assert!(state.inventory.current);
    assert!(!refreshing(&state));
    assert_eq!(state.grant(address, Instant::now()), Some(grant));
}

#[test]
fn a_full_refresh_queue_cannot_leave_the_previous_catalogue_presented_as_current() {
    let mut state = crate::client_capability_support::state();
    state.control_generation = Some(Uuid::new_v4());
    state.inventory.current = true;
    let (jobs, _incoming) = mpsc::channel(ONE_JOB);
    catalog_events::refresh(&mut state, &jobs);
    let previous = state.catalog_pending.unwrap();
    inventory_actions::handle(&mut state, &jobs, KeyCode::Char(REFRESH_KEY));
    assert!(state.catalog_pending.is_none());
    assert!(!state.inventory.current);
    assert!(refreshing(&state));
    assert_eq!(state.status, QUEUE_FULL_LABEL);
    catalog_events::apply(
        &mut state,
        CatalogEvent {
            request_id: previous,
            result: Ok(Catalog::default()),
        },
    );
    assert!(!state.inventory.current);
    state.control_generation = None;
    assert!(!refreshing(&state));
}

#[test]
fn a_recovered_catalogue_failure_does_not_leave_a_stale_transport_error() {
    let mut state = crate::client_capability_support::state();
    state.control_generation = Some(Uuid::new_v4());
    let request_id = Uuid::new_v4();
    state.catalog_pending = Some(request_id);
    catalog_events::apply(
        &mut state,
        CatalogEvent {
            request_id,
            result: Err(CatalogError::Limit),
        },
    );
    let failure = state.status.clone();
    assert!(!failure.is_empty());
    assert_eq!(state.recoverable_status.as_deref(), Some(failure.as_str()));
    assert!(!state.inventory.current);

    state.catalog_pending = Some(request_id);
    catalog_events::apply(
        &mut state,
        CatalogEvent {
            request_id,
            result: Ok(Catalog::default()),
        },
    );
    assert!(state.inventory.current);
    assert_eq!(state.recoverable_status, None);
    assert_ne!(state.status, failure);
}

#[test]
fn a_transient_transport_failure_clears_after_the_next_catalogue_refresh() {
    let mut state = crate::client_capability_support::state();
    state.control_generation = Some(Uuid::new_v4());
    let message = "Hub transport failed: Hub did not respond in time; retry shortly";
    let (jobs, _incoming) = mpsc::channel(QUEUE_CAPACITY);
    crate::events::apply(
        &mut state,
        &jobs,
        crate::network::Update::Failed {
            command: None,
            message: message.into(),
            connected: true,
            transient: true,
        },
    );
    assert_eq!(state.status, message);
    assert_eq!(state.recoverable_status.as_deref(), Some(message));
    let request_id = Uuid::new_v4();
    state.catalog_pending = Some(request_id);
    catalog_events::apply(
        &mut state,
        CatalogEvent {
            request_id,
            result: Ok(Catalog::default()),
        },
    );
    assert_ne!(state.status, message);
    assert_eq!(state.recoverable_status, None);
}


include!("catalog_refresh_recovery_status.rs");

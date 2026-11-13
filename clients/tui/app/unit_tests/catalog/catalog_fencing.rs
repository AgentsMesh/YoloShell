use super::fixtures;
use crate::{
    catalog_events, events,
    network::{Job, Update},
};
use tokio::sync::mpsc;
use uuid::Uuid;
use yolo_hub_catalog_client::{Catalog, CatalogError};
use yolo_hub_client_channels::CatalogEvent;

fn success(request_id: Uuid, session: yolo_hub_terminal_protocol::TerminalSummary) -> Update {
    Update::Catalog(CatalogEvent {
        request_id,
        result: Ok(Catalog {
            terminals: vec![session],
            ..Catalog::default()
        }),
    })
}

#[test]
fn catalogue_success_is_not_control_readiness_and_failure_does_not_revoke_a_live_writer() {
    let session = fixtures::session();
    let address = session.fields().address;
    let mut state = crate::client_capability_support::state();
    let (jobs, mut received) = mpsc::channel(crate::constants::QUEUE_CAPACITY);
    catalog_events::refresh(&mut state, &jobs);
    let request_id = state.catalog_pending.unwrap();
    received.try_recv().unwrap();
    events::apply(&mut state, &jobs, success(request_id, session.clone()));
    assert!(state.inventory.current);
    assert!(state.control_generation.is_none());
    let generation = Uuid::new_v4();
    state.control_generation = Some(generation);
    let grant = fixtures::grant(&session);
    state.record_grant(grant, std::time::Instant::now());
    catalog_events::refresh(&mut state, &jobs);
    let request_id = state.catalog_pending.unwrap();
    events::apply(
        &mut state,
        &jobs,
        Update::Catalog(CatalogEvent {
            request_id,
            result: Err(CatalogError::Limit),
        }),
    );
    assert!(!state.inventory.current);
    assert_eq!(state.control_generation, Some(generation));
    assert_eq!(state.grant(address, std::time::Instant::now()), Some(grant));
}

#[test]
fn late_success_and_failure_cannot_retire_or_overwrite_a_newer_refresh() {
    let previous = fixtures::session();
    let current = fixtures::session();
    let mut state = crate::client_capability_support::state();
    let (jobs, _received) = mpsc::channel(crate::constants::QUEUE_CAPACITY);
    catalog_events::refresh(&mut state, &jobs);
    let old = state.catalog_pending.unwrap();
    catalog_events::invalidate(&mut state, &jobs);
    let new = state.catalog_pending.unwrap();
    assert_ne!(old, new);
    events::apply(&mut state, &jobs, success(old, previous.clone()));
    events::apply(
        &mut state,
        &jobs,
        Update::Catalog(CatalogEvent {
            request_id: old,
            result: Err(CatalogError::Limit),
        }),
    );
    assert_eq!(state.catalog_pending, Some(new));
    assert!(state.terminals.is_empty());
    events::apply(&mut state, &jobs, success(new, current.clone()));
    events::apply(&mut state, &jobs, success(old, previous));
    assert!(state.catalog_pending.is_none());
    assert_eq!(state.terminals.values().collect::<Vec<_>>(), vec![&current]);
}

#[test]
fn invalidation_remains_effective_when_the_job_queue_is_full() {
    let mut state = crate::client_capability_support::state();
    let (jobs, mut received) = mpsc::channel(SINGLE_JOB);
    catalog_events::refresh(&mut state, &jobs);
    let old = state.catalog_pending.unwrap();
    state.inventory.current = true;
    catalog_events::invalidate(&mut state, &jobs);
    assert!(state.catalog_pending.is_none());
    assert!(!state.inventory.current);
    events::apply(&mut state, &jobs, success(old, fixtures::session()));
    assert!(state.terminals.is_empty());
    assert!(
        matches!(received.try_recv().unwrap(), Job::Refresh { request_id } if request_id == old)
    );
}

const SINGLE_JOB: usize = 1;

#[test]
fn a_control_observation_of_a_new_runtime_cannot_be_reversed_by_an_older_catalogue() {
    use yolo_hub_client_channels::{ControlEvent, ControlRequest};
    use yolo_hub_rpc_protocol::{Command, Reply};
    let original = fixtures::session();
    let mut fields = original.fields().clone();
    fields.runtime.as_mut().unwrap().target.runtime_epoch = yolo_hub_identity::RuntimeEpoch::new();
    let current = yolo_hub_terminal_protocol::TerminalSummary::new(fields).unwrap();
    let generation = Uuid::new_v4();
    let mut state = crate::client_capability_support::state();
    state.control_generation = Some(generation);
    let old = Uuid::new_v4();
    state.catalog_pending = Some(old);
    state.replace_terminals(vec![original.clone()]);
    let (jobs, _received) = mpsc::channel(crate::constants::QUEUE_CAPACITY);
    events::apply(
        &mut state,
        &jobs,
        Update::Control(ControlEvent::Finished {
            request: ControlRequest {
                validity: None,
                generation: Some(generation),
                destination: current.fields().address.hub_id,
                command: Command::Get {
                    address: current.fields().address,
                },
            },
            result: Ok(Reply::Terminal {
                terminal: current.clone(),
            }),
            connected: true,
        }),
    );
    let pending = state.catalog_pending;
    assert_ne!(pending, Some(old));
    events::apply(&mut state, &jobs, success(old, original));
    assert_eq!(state.catalog_pending, pending);
    assert_eq!(state.terminals[&current.fields().address], current);
}

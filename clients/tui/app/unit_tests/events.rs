use super::fixtures::*;
use crate::{
    events::{apply, enqueue, refresh},
    network::{Job, Update},
};
use tokio::sync::mpsc;
use yolo_hub_rpc_protocol::{Command, Reply};
use yolo_hub_terminal_protocol::{OutputCursor, TerminalEvent};

#[test]
fn bounded_command_queue_does_not_silently_discard_control_and_refresh_is_coalesced() {
    let mut state = crate::client_capability_support::state();
    let (jobs, mut incoming) = mpsc::channel(1);
    refresh(&mut state, &jobs);
    assert!(state.catalog_pending.is_some());
    let request_id = state.catalog_pending.unwrap();
    refresh(&mut state, &jobs);
    assert!(!enqueue(&mut state, &jobs, Job::Refresh { request_id }));
    assert!(
        matches!(incoming.try_recv().unwrap(), Job::Refresh { request_id: actual } if actual == request_id)
    );
    assert!(incoming.try_recv().is_err());
    apply(
        &mut state,
        &jobs,
        Update::Catalog(yolo_hub_client_channels::CatalogEvent {
            request_id,
            result: Ok(yolo_hub_catalog_client::Catalog {
                terminals: vec![session()],
                ..Default::default()
            }),
        }),
    );
    assert!(state.control_generation.is_none());
    assert!(state.catalog_pending.is_none());
    assert_eq!(state.terminals.len(), 1);
}

#[test]
fn platform_event_wake_invalidates_and_requests_a_fresh_catalogue() {
    let mut state = crate::client_capability_support::state();
    let (jobs, mut incoming) = mpsc::channel(2);
    apply(
        &mut state,
        &jobs,
        Update::PlatformEvents(yolo_hub_client_channels::PlatformEventWake {
            lost: true,
            layout_changed: false,
        }),
    );
    let request_id = state.catalog_pending.unwrap();
    assert!(!state.inventory.current);
    assert!(
        matches!(incoming.try_recv().unwrap(), Job::Refresh { request_id: actual } if actual == request_id)
    );
}

#[test]
fn late_grant_from_replaced_runtime_does_not_restore_control() {
    let old = session();
    let grant = grant(&old);
    let mut state = crate::client_capability_support::state();
    let (jobs, _) = mpsc::channel(1);
    state.replace_terminals(vec![session()]);
    apply(
        &mut state,
        &jobs,
        Update::Reply {
            command: Command::Renew { grant },
            reply: Reply::Lease { grant },
        },
    );
    assert!(state.writers.is_empty());
}

#[test]
fn get_reply_retires_writer_and_surface_when_runtime_is_replaced() {
    let original = session();
    let grant = grant(&original);
    let mut fields = original.fields().clone();
    fields.runtime.as_mut().unwrap().target.runtime_epoch = yolo_hub_identity::RuntimeEpoch::new();
    let replacement = yolo_hub_terminal_protocol::TerminalSummary::new(fields).unwrap();
    let mut state = crate::client_capability_support::state();
    state.replace_terminals(vec![original.clone()]);
    state.record_grant(grant, std::time::Instant::now());
    let target = original.fields().runtime.unwrap().target;
    state.surfaces.insert(
        target.address,
        yolo_tui_terminal::TerminalSurface::new(&super::query_support::snapshot(target, 1, b"")),
    );
    let (jobs, _) = mpsc::channel(crate::constants::QUEUE_CAPACITY);
    apply(
        &mut state,
        &jobs,
        Update::Reply {
            command: Command::Get {
                address: replacement.fields().address,
            },
            reply: Reply::Terminal {
                terminal: replacement.clone(),
            },
        },
    );
    assert!(state.writers.is_empty());
    assert!(state.surfaces.is_empty());
    assert_eq!(state.terminals[&target.address], replacement);
}

#[test]
fn stream_failure_marks_only_the_origin_session_and_never_synthesizes_exit() {
    let current = session();
    let target = current.fields().runtime.unwrap().target;
    let mut state = crate::client_capability_support::state();
    state.replace_terminals(vec![current]);
    crate::observation_events::closed(&mut state, target, Some(String::from("Disconnected")));
    assert!(state.is_offline(target.address));
    assert!(!state.ended_epoch(target.address).is_some());
    crate::observation_events::terminal(
        &mut state,
        TerminalEvent::Exited {
            cursor: OutputCursor {
                target,
                after_sequence: 0,
            },
            code: Some(0),
            signal: None,
        },
    );
    assert_eq!(
        state.ended_epoch(target.address),
        Some(target.runtime_epoch)
    );
}

#[test]
fn failed_network_update_removes_local_write_authority() {
    let current = session();
    let grant = grant(&current);
    let mut state = crate::client_capability_support::state();
    let (jobs, _) = mpsc::channel(1);
    state.record_grant(grant, std::time::Instant::now());
    apply(
        &mut state,
        &jobs,
        Update::Failed {
            command: None,
            message: String::from("Disconnected"),
            connected: false,
            transient: false,
        },
    );
    assert!(state.control_generation.is_none());
    assert!(state.writers.is_empty());
}

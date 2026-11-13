use super::fixtures;
use crate::{
    events,
    network::{Job, Update},
};
use tokio::sync::mpsc;
use uuid::Uuid;
use yolo_hub_client_channels::{ControlError, ControlEvent, ControlRequest};
use yolo_hub_rpc_protocol::{Command, Reply};

#[test]
fn connection_ready_retires_previous_authority_and_stamps_new_commands() {
    let session = fixtures::session();
    let grant = fixtures::grant(&session);
    let old = Uuid::new_v4();
    let new = Uuid::new_v4();
    let mut state = crate::client_capability_support::state();
    state.control_generation = Some(old);
    state.record_grant(grant, std::time::Instant::now());
    let (jobs, mut received) = mpsc::channel(crate::constants::QUEUE_CAPACITY);
    events::apply(
        &mut state,
        &jobs,
        Update::Control(crate::client_capability_support::ready(new)),
    );
    assert!(state.writers.is_empty());
    assert_eq!(state.control_generation, Some(new));
    assert!(matches!(received.try_recv().unwrap(), Job::Refresh { .. }));
    events::enqueue(
        &mut state,
        &jobs,
        Job::Call {
            generation: Some(old),
            origin: session.fields().address.hub_id,
            command: Command::Renew { grant },
        },
    );
    assert!(
        matches!(received.try_recv().unwrap(), Job::Call { generation: Some(actual), .. } if actual == new)
    );
    events::apply(
        &mut state,
        &jobs,
        Update::Control(crate::client_capability_support::ready(new)),
    );
    events::apply(
        &mut state,
        &jobs,
        Update::Control(crate::client_capability_support::ready(Uuid::nil())),
    );
    assert!(received.try_recv().is_err());
    assert_eq!(state.control_generation, Some(new));
}

#[test]
fn stale_control_completion_cannot_restore_writers_or_disconnect_a_new_connection() {
    let session = fixtures::session();
    let grant = fixtures::grant(&session);
    let mut state = crate::client_capability_support::state();
    let new = Uuid::new_v4();
    state.control_generation = Some(new);
    state.replace_terminals(vec![session.clone()]);
    let (jobs, _received) = mpsc::channel(crate::constants::QUEUE_CAPACITY);
    for result in [Ok(Reply::Lease { grant }), Err(ControlError::NotConnected)] {
        events::apply(
            &mut state,
            &jobs,
            Update::Control(ControlEvent::Finished {
                request: ControlRequest {
                    validity: None,
                    generation: Some(Uuid::new_v4()),
                    destination: session.fields().address.hub_id,
                    command: Command::Renew { grant },
                },
                result,
                connected: false,
            }),
        );
    }
    assert!(state.writers.is_empty());
    assert_eq!(state.control_generation, Some(new));
}

#[test]
fn structural_completion_invalidates_older_catalog_even_when_delivery_is_uncertain() {
    let session = fixtures::session();
    let grant = fixtures::grant(&session);
    let generation = Uuid::new_v4();
    let mut state = crate::client_capability_support::state();
    state.control_generation = Some(generation);
    let old = Uuid::new_v4();
    state.catalog_pending = Some(old);
    state.inventory.current = true;
    let (jobs, mut received) = mpsc::channel(crate::constants::QUEUE_CAPACITY);
    events::apply(
        &mut state,
        &jobs,
        Update::Control(ControlEvent::Finished {
            request: ControlRequest {
                validity: None,
                generation: Some(generation),
                destination: session.fields().address.hub_id,
                command: Command::Resize {
                    request: yolo_hub_terminal_protocol::ResizeRequest {
                        request_id: Uuid::new_v4(),
                        grant,
                        size: yolo_hub_terminal_protocol::TerminalSize::default(),
                    },
                },
            },
            result: Err(ControlError::Client(
                yolo_hub_client::ClientError::OutcomeUnknownResponse(
                    yolo_hub_terminal_protocol::PublicError::InvalidPayload,
                ),
            )),
            connected: false,
        }),
    );
    assert!(state.control_generation.is_none());
    assert!(!state.inventory.current);
    assert_ne!(state.catalog_pending, Some(old));
    assert!(
        matches!(received.try_recv().unwrap(), Job::Refresh { request_id } if Some(request_id) == state.catalog_pending)
    );
}

#[test]
fn unreachable_connection_does_not_mark_independent_catalogue_as_stale() {
    let mut state = crate::client_capability_support::state();
    state.inventory.current = true;
    let (jobs, _received) = mpsc::channel(crate::constants::QUEUE_CAPACITY);
    events::apply(
        &mut state,
        &jobs,
        Update::Control(ControlEvent::Unreachable {
            generation: None,
            message: crate::constants::DISCONNECTED_LABEL.into(),
        }),
    );
    assert!(state.inventory.current);
    assert_eq!(state.status, crate::constants::DISCONNECTED_LABEL);
    state.control_generation = Some(Uuid::new_v4());
    state.status = crate::constants::CONNECTED_LABEL.into();
    events::apply(
        &mut state,
        &jobs,
        Update::Control(ControlEvent::Unreachable {
            generation: None,
            message: crate::constants::DISCONNECTED_LABEL.into(),
        }),
    );
    assert_eq!(state.status, crate::constants::CONNECTED_LABEL);
}

#[test]
fn failed_keepalive_retires_only_the_matching_control_generation() {
    let session = fixtures::session();
    let grant = fixtures::grant(&session);
    let current = Uuid::new_v4();
    let mut state = crate::client_capability_support::state();
    state.control_generation = Some(current);
    state.inventory.current = true;
    state.record_grant(grant, std::time::Instant::now());
    let (jobs, _received) = mpsc::channel(crate::constants::QUEUE_CAPACITY);
    for generation in [Some(Uuid::new_v4()), None] {
        events::apply(
            &mut state,
            &jobs,
            Update::Control(ControlEvent::Unreachable {
                generation,
                message: crate::constants::DISCONNECTED_LABEL.into(),
            }),
        );
        assert_eq!(state.control_generation, Some(current));
        assert!(!state.writers.is_empty());
    }
    events::apply(
        &mut state,
        &jobs,
        Update::Control(ControlEvent::Unreachable {
            generation: Some(current),
            message: crate::constants::DISCONNECTED_LABEL.into(),
        }),
    );
    assert!(state.control_generation.is_none());
    assert!(state.writers.is_empty());
    assert!(state.inventory.current);
}

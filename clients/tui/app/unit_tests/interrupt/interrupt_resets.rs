use super::{fixtures, interrupt_support::*};
use crate::{
    constants::QUEUE_CAPACITY,
    events,
    interrupt_actions::status,
    interrupt_constants as text,
    interrupt_state::Phase,
    network::{Job, Update},
};
use crossterm::event::KeyEventKind;
use std::time::Instant;
use tokio::sync::mpsc;
use yolo_hub_agent_protocol::{AgentControlRequest, CommandOutcome};
use yolo_hub_client_channels::{ControlError, ControlEvent};
use yolo_hub_rpc_protocol::Command;

#[test]
fn channel_reset_preserves_unknown_and_late_correlated_result_cannot_disconnect_new_authority() {
    let mut state = ready();
    let (jobs, mut incoming) = mpsc::channel(QUEUE_CAPACITY);
    press(&mut state, &jobs, KeyEventKind::Press);
    let old = queued(&mut incoming);
    let fresh = uuid::Uuid::new_v4();
    events::apply(
        &mut state,
        &jobs,
        Update::Control(crate::client_capability_support::ready(fresh)),
    );
    assert_eq!(status(&state), text::UNKNOWN);
    assert!(matches!(incoming.try_recv().unwrap(), Job::Refresh { .. }));
    let session = state.terminals.values().next().unwrap().clone();
    let grant = fixtures::grant(&session);
    state.principal = grant.fields().principal;
    state.record_grant(grant, Instant::now());
    state.inventory.current = true;
    press(&mut state, &jobs, KeyEventKind::Press);
    assert!(incoming.try_recv().is_err());
    events::apply(
        &mut state,
        &jobs,
        Update::Control(ControlEvent::Finished {
            request: old.clone(),
            result: Ok(reply(&old, CommandOutcome::Delivered {})),
            connected: false,
        }),
    );
    assert_eq!(state.control_generation, Some(fresh));
    assert_eq!(
        state.grant(session.fields().address, Instant::now()),
        Some(grant)
    );
    assert_eq!(status(&state), text::SENT);
}

#[test]
fn stale_uncorrelated_failure_cannot_clear_current_writer_or_replace_pending_request() {
    let mut state = ready();
    let (jobs, mut incoming) = mpsc::channel(QUEUE_CAPACITY);
    press(&mut state, &jobs, KeyEventKind::Press);
    let current = queued(&mut incoming);
    let mut stale = current.clone();
    let Command::AgentControl {
        request: stale_request,
        ..
    } = &mut stale.command
    else {
        panic!()
    };
    let mut fields = stale_request.fields().clone();
    fields.request_id = uuid::Uuid::new_v4();
    *stale_request = AgentControlRequest::new(fields).unwrap();
    let generation = state.control_generation;
    events::apply(
        &mut state,
        &jobs,
        Update::Control(ControlEvent::Finished {
            request: stale,
            result: Err(ControlError::NotConnected),
            connected: false,
        }),
    );
    assert_eq!(state.control_generation, generation);
    assert!(!state.writers.is_empty());
    assert_eq!(
        state.interrupts.get(target(&state)).unwrap().phase,
        Phase::Sending
    );
    assert_eq!(
        state.interrupts.get(target(&state)).unwrap().request,
        *request(&current)
    );
}

#[test]
fn delayed_notsent_retires_only_its_exact_old_generation_request_after_reconnect() {
    let mut state = ready();
    let (jobs, mut incoming) = mpsc::channel(QUEUE_CAPACITY);
    press(&mut state, &jobs, KeyEventKind::Press);
    let old = queued(&mut incoming);
    let fresh = uuid::Uuid::new_v4();
    events::apply(
        &mut state,
        &jobs,
        Update::Control(crate::client_capability_support::ready(fresh)),
    );
    events::apply(&mut state, &jobs, Update::NotSent { request: old });
    assert_eq!(state.control_generation, Some(fresh));
    assert_eq!(status(&state), text::NOT_SENT);
    assert_eq!(
        state.interrupts.get(target(&state)).unwrap().phase,
        Phase::NotSent
    );
    assert!(matches!(incoming.try_recv().unwrap(), Job::Refresh { .. }));
    assert!(incoming.try_recv().is_err());
}

#[test]
fn lost_channel_marks_pending_unknown_and_ready_does_not_replay_it() {
    let mut state = ready();
    let (jobs, mut incoming) = mpsc::channel(QUEUE_CAPACITY);
    press(&mut state, &jobs, KeyEventKind::Press);
    let pending = queued(&mut incoming);
    let generation = state.control_generation;
    events::apply(
        &mut state,
        &jobs,
        Update::Control(ControlEvent::Unreachable {
            generation,
            message: PRIVATE_MESSAGE.into(),
        }),
    );
    assert_eq!(status(&state), text::UNKNOWN);
    assert!(state.writers.is_empty());
    events::apply(
        &mut state,
        &jobs,
        Update::Control(crate::client_capability_support::ready(uuid::Uuid::new_v4())),
    );
    assert!(matches!(incoming.try_recv().unwrap(), Job::Refresh { .. }));
    assert!(incoming.try_recv().is_err());
    assert_eq!(
        state.interrupts.get(target(&state)).unwrap().request,
        *request(&pending)
    );
}

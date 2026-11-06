use super::interrupt_support::*;
use crate::{constants::QUEUE_CAPACITY, interrupt_constants as text, network::Job};
use crossterm::event::KeyEventKind;
use std::time::Instant;
use tokio::sync::mpsc;
use yolo_hub_agent_protocol::{AgentAction, Capability, Freshness, Reachability};
use yolo_hub_identity::{HubId, RuntimeEpoch};
use yolo_hub_rpc_protocol::Command;
use yolo_hub_terminal_protocol::LeaseGrant;

#[test]
fn press_enqueues_scoped_agent_control_with_existing_exact_writer_and_no_pty_input() {
    let mut state = ready();
    let target = target(&state);
    let expected = state.inventory.agents[&target].cursor();
    let grant = state.writers.values().next().unwrap().grant;
    let (jobs, mut incoming) = mpsc::channel(QUEUE_CAPACITY);
    press(&mut state, &jobs, KeyEventKind::Press);
    let queued = queued(&mut incoming);
    assert_eq!(queued.destination, target.address.hub_id);
    assert_eq!(queued.generation, state.control_generation);
    let Command::AgentControl {
        request,
        grant: actual,
    } = queued.command
    else {
        panic!()
    };
    assert_eq!(request.fields().target, target);
    assert_eq!(request.fields().expected_cursor, expected);
    assert_eq!(request.fields().action, AgentAction::Interrupt {});
    assert_eq!(actual, grant);
    assert_eq!(state.status, text::SENDING);
    assert!(incoming.try_recv().is_err());
}

#[test]
fn repeat_release_and_duplicate_press_while_pending_never_send_another_effect() {
    let mut state = ready();
    let (jobs, mut incoming) = mpsc::channel(QUEUE_CAPACITY);
    for kind in [KeyEventKind::Repeat, KeyEventKind::Release] {
        press(&mut state, &jobs, kind);
    }
    assert!(incoming.try_recv().is_err());
    press(&mut state, &jobs, KeyEventKind::Press);
    let first = queued(&mut incoming);
    for kind in [
        KeyEventKind::Press,
        KeyEventKind::Press,
        KeyEventKind::Repeat,
    ] {
        press(&mut state, &jobs, kind);
    }
    assert!(incoming.try_recv().is_err());
    assert_eq!(
        state.interrupts.get(target(&state)).unwrap().request,
        *request(&first)
    );
}

#[test]
fn readonly_expired_or_wrong_runtime_writer_requires_explicit_control_without_acquisition() {
    for invalid in [
        MissingWriter::Absent,
        MissingWriter::Expired,
        MissingWriter::Runtime,
        MissingWriter::Principal,
    ] {
        let mut state = ready();
        match invalid {
            MissingWriter::Absent => state.writers.clear(),
            MissingWriter::Expired => {
                state.writers.values_mut().next().unwrap().expires = Instant::now()
            }
            MissingWriter::Runtime | MissingWriter::Principal => {
                let writer = state.writers.values_mut().next().unwrap();
                let mut fields = *writer.grant.fields();
                if matches!(invalid, MissingWriter::Runtime) {
                    fields.target.runtime_epoch = RuntimeEpoch::new();
                } else {
                    fields.principal.client_id = uuid::Uuid::new_v4();
                }
                writer.grant = LeaseGrant::new(fields).unwrap();
            }
        }
        let (jobs, mut incoming) = mpsc::channel(QUEUE_CAPACITY);
        press(&mut state, &jobs, KeyEventKind::Press);
        assert_eq!(state.status, text::TAKE_CONTROL);
        assert!(incoming.try_recv().is_err());
    }
}
#[derive(Clone, Copy)]
enum MissingWriter {
    Absent,
    Expired,
    Runtime,
    Principal,
}

#[test]
fn stale_unreachable_unsupported_and_unselected_agents_cannot_trigger_control() {
    let mutations: &[fn(&mut crate::State)] = &[
        |s| s.inventory.current = false,
        |s| mutate_agent(s, |f| f.freshness = Freshness::Stale),
        |s| mutate_agent(s, |f| f.reachability = Reachability::Unreachable),
        |s| mutate_agent(s, |f| f.capabilities.interrupt = Capability::Unsupported {}),
        |s| s.inventory.selected_agent = None,
    ];
    for mutate in mutations {
        let mut state = ready();
        mutate(&mut state);
        let (jobs, mut incoming) = mpsc::channel(QUEUE_CAPACITY);
        press(&mut state, &jobs, KeyEventKind::Press);
        assert!(incoming.try_recv().is_err());
        assert!(state.interrupts.get(target(&state)).is_none());
    }
}

#[test]
fn full_ui_queue_does_not_leave_a_pending_effect_or_implicitly_take_control() {
    let mut state = ready();
    let (jobs, mut incoming) = mpsc::channel(SINGLE_JOB);
    jobs.try_send(Job::Refresh {
        request_id: uuid::Uuid::new_v4(),
    })
    .ok()
    .unwrap();
    press(&mut state, &jobs, KeyEventKind::Press);
    assert_eq!(state.status, text::NOT_SENT);
    assert!(state.interrupts.get(target(&state)).is_none());
    assert!(matches!(incoming.try_recv().unwrap(), Job::Refresh { .. }));
    press(&mut state, &jobs, KeyEventKind::Press);
    queued(&mut incoming);
}

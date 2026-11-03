use super::fixtures;
use crate::{State, constants::QUEUE_FULL_LABEL, network::Job, polling};
use std::time::Instant;
use tokio::sync::mpsc;
use yolo_hub_identity::TerminalAddress;
use yolo_hub_rpc_protocol::Command;
use yolo_hub_terminal_protocol::{LeaseGrant, TerminalSize};
use yolo_presentation_layout::Axis;

const TEST_QUEUE_CAPACITY: usize = 1;
const SHARED_PANES: usize = 2;
const REMAINING_PANES: usize = 1;

fn bound_writer() -> (State, TerminalAddress, LeaseGrant) {
    let session = fixtures::session();
    let address = session.fields().address;
    let grant = fixtures::grant(&session);
    let mut state = crate::client_capability_support::state();
    state.control_generation = Some(uuid::Uuid::new_v4());
    state.replace_terminals(vec![session]);
    state.bind_focused(Some(address)).unwrap();
    state.record_grant(grant, Instant::now());
    (state, address, grant)
}

#[test]
fn closing_one_shared_pane_keeps_the_visible_sessions_writer() {
    let (mut state, address, grant) = bound_writer();
    let second = state
        .tab_layout_mut()
        .split(Axis::Horizontal, Some(address))
        .unwrap();
    assert_eq!(state.tab_layout().panes().len(), SHARED_PANES);
    state.tab_layout_mut().close(second).unwrap();
    assert_eq!(state.tab_layout().panes().len(), REMAINING_PANES);
    assert_eq!(state.focused_terminal(), Some(address));
    let (jobs, mut received) = mpsc::channel(TEST_QUEUE_CAPACITY);
    polling::renew_leases(&mut state, &jobs);
    assert_eq!(state.grant(address, Instant::now()), Some(grant));
    assert!(received.try_recv().is_err());
    state.writers.get_mut(&address).unwrap().renew = Instant::now();
    polling::renew_leases(&mut state, &jobs);
    assert!(matches!(received.try_recv().unwrap(), Job::Call {
        command: Command::Renew { grant: renewed }, ..
    } if renewed == grant));
    assert!(state.renewal_pending(address));
}

#[test]
fn acquiring_an_already_held_writer_does_not_enqueue_duplicate_authority() {
    let (mut state, address, grant) = bound_writer();
    let (jobs, mut received) = mpsc::channel(TEST_QUEUE_CAPACITY);
    polling::acquire(&mut state, &jobs, false);
    polling::acquire(&mut state, &jobs, false);
    assert!(received.try_recv().is_err());
    assert_eq!(state.grant(address, Instant::now()), Some(grant));
    assert!(!state.has_renewals_pending());
}

#[test]
fn unbinding_retires_pending_renewal_and_size_before_the_next_poll() {
    let (mut state, address, grant) = bound_writer();
    state.mark_renewal_pending(address);
    state.set_last_size(address, TerminalSize::default());
    state.writers.get_mut(&address).unwrap().renew = Instant::now();
    state.bind_focused(None).unwrap();
    let (jobs, mut received) = mpsc::channel(TEST_QUEUE_CAPACITY);
    polling::renew_leases(&mut state, &jobs);
    assert!(!state.writers.contains_key(&address));
    assert!(!state.renewal_pending(address));
    assert!(!state.last_size(address).is_some());
    assert!(matches!(received.try_recv().unwrap(), Job::Call {
        origin, command: Command::Release { grant: released }, ..
    } if origin == address.hub_id && released == grant));
    polling::renew_leases(&mut state, &jobs);
    assert!(received.try_recv().is_err());
}

#[test]
fn full_job_queue_cannot_preserve_or_renew_a_hidden_writer() {
    let (mut state, address, _) = bound_writer();
    state.mark_renewal_pending(address);
    state.set_last_size(address, TerminalSize::default());
    state.writers.get_mut(&address).unwrap().renew = Instant::now();
    state.bind_focused(None).unwrap();
    let (jobs, mut received) = mpsc::channel(TEST_QUEUE_CAPACITY);
    assert!(
        jobs.try_send(Job::Refresh {
            request_id: uuid::Uuid::new_v4()
        })
        .is_ok()
    );
    polling::renew_leases(&mut state, &jobs);
    assert_eq!(state.status, QUEUE_FULL_LABEL);
    assert!(state.grant(address, Instant::now()).is_none());
    assert!(!state.writers.contains_key(&address));
    assert!(!state.renewal_pending(address));
    assert!(!state.last_size(address).is_some());
    assert!(matches!(received.try_recv().unwrap(), Job::Refresh { .. }));
    polling::renew_leases(&mut state, &jobs);
    assert!(received.try_recv().is_err());
}

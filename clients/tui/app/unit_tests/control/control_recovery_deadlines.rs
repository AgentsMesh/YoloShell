//! Child tests of the control adapter, so deadlines can be advanced without
//! sleeping or exposing timer mutation in the application's public interface.

#[path = "../../test_support/fixtures.rs"]
mod fixtures;

use crate::network::Job;
use std::time::Instant;
use tokio::sync::mpsc;
use uuid::Uuid;
use yolo_hub_terminal_protocol::LeaseMode;

#[test]
fn lost_takeover_observation_times_out_and_only_ordinary_acquisition_can_follow() {
    let terminal = fixtures::session();
    let target = terminal.fields().runtime.unwrap().target;
    let mut state = crate::client_capability_support::state();
    state.control_generation = Some(Uuid::new_v4());
    state.replace_terminals(vec![terminal]);
    state.bind_focused(Some(target.address)).unwrap();
    let (jobs, mut incoming) = mpsc::channel(crate::constants::QUEUE_CAPACITY);
    super::takeover(&mut state, &jobs, target);
    let Job::Scoped(observation) = incoming.try_recv().unwrap() else {
        panic!()
    };
    super::reconcile(&mut state, &jobs);
    assert!(incoming.try_recv().is_err());
    assert!(observation.validity.as_ref().unwrap().is_current());

    // Advance only the private observation deadline. The retained transport
    // request remains live until reconciliation explicitly retires its scope.
    state
        .control
        .entries
        .get_mut(&target.address)
        .unwrap()
        .takeover
        .as_mut()
        .unwrap()
        .1 = Instant::now();
    super::reconcile(&mut state, &jobs);
    assert!(!observation.validity.as_ref().unwrap().is_current());
    assert!(!super::observe_reply(&mut state, &observation));
    assert!(incoming.try_recv().is_err());
    assert_eq!(
        state.control.entries[&target.address].policy.failure(),
        Some(yolo_tui_terminal_control::Failure::Transient)
    );

    super::acquire(&mut state, &jobs, target);
    let Job::Acquire {
        request, validity, ..
    } = incoming.try_recv().unwrap()
    else {
        panic!()
    };
    assert_eq!(request.target, target);
    assert_eq!(request.mode, LeaseMode::Acquire {});
    assert!(validity.is_current());
    assert!(state.writers.is_empty());
    assert!(incoming.try_recv().is_err());
}

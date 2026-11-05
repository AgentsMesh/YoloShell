use crate::{network::Update, update_queue::drain_bounded};
use tokio::sync::mpsc;

fn failed(message: &str) -> Update {
    Update::Failed {
        command: None,
        message: message.to_owned(),
        connected: true,
        transient: false,
    }
}

fn taken(receiver: &mut mpsc::Receiver<Update>) -> String {
    match receiver.try_recv().expect("expected another queued update") {
        Update::Failed { message, .. } => message,
        _ => panic!("this fixture queues only failure updates"),
    }
}

#[test]
fn priority_updates_are_applied_before_lower_urgency_updates() {
    let mut state = crate::client_capability_support::state();
    let (jobs, _incoming) = mpsc::channel(1);
    let (priority_tx, mut priority) = mpsc::channel(4);
    let (normal_tx, mut normal) = mpsc::channel(4);
    let (observation_tx, mut observations) = mpsc::channel(4);
    priority_tx.try_send(failed("priority")).unwrap();
    normal_tx.try_send(failed("normal")).unwrap();
    observation_tx.try_send(failed("observation")).unwrap();

    drain_bounded(
        &mut state,
        &jobs,
        &mut priority,
        &mut normal,
        &mut observations,
        1,
    );

    assert_eq!(state.status, "priority");
    assert_eq!(taken(&mut normal), "normal");
    assert_eq!(taken(&mut observations), "observation");
}

#[test]
fn coalescing_applies_exactly_the_budget_in_priority_order() {
    let mut state = crate::client_capability_support::state();
    let (jobs, _incoming) = mpsc::channel(1);
    let (priority_tx, mut priority) = mpsc::channel(8);
    let (normal_tx, mut normal) = mpsc::channel(8);
    let (observation_tx, mut observations) = mpsc::channel(8);
    for index in 0..3 {
        priority_tx
            .try_send(failed(&format!("priority-{index}")))
            .unwrap();
    }
    for index in 0..2 {
        normal_tx
            .try_send(failed(&format!("normal-{index}")))
            .unwrap();
    }
    observation_tx.try_send(failed("observation")).unwrap();

    drain_bounded(
        &mut state,
        &jobs,
        &mut priority,
        &mut normal,
        &mut observations,
        4,
    );

    // All three priority updates ran before the first normal one, and the
    // observation lane was not reached within the budget.
    assert_eq!(state.status, "normal-0");
    assert!(matches!(
        priority.try_recv(),
        Err(mpsc::error::TryRecvError::Empty)
    ));

    drain_bounded(
        &mut state,
        &jobs,
        &mut priority,
        &mut normal,
        &mut observations,
        1,
    );
    assert_eq!(state.status, "normal-1");

    drain_bounded(
        &mut state,
        &jobs,
        &mut priority,
        &mut normal,
        &mut observations,
        1,
    );
    assert_eq!(state.status, "observation");
    assert!(matches!(
        normal.try_recv(),
        Err(mpsc::error::TryRecvError::Empty)
    ));
    assert!(matches!(
        observations.try_recv(),
        Err(mpsc::error::TryRecvError::Empty)
    ));
}

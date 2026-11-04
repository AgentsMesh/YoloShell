use super::{creation_support::Fixture, finalization_support::*};
use crate::{events, network::Update};
use yolo_hub_client_channels::ControlError;
use yolo_hub_rpc_protocol::{Command, Reply};

#[test]
fn lost_owner_completion_retries_only_after_deadline_and_keeps_late_success_safe() {
    let mut fixture = Fixture::new();
    let (_, _, lost) = confirmed(&mut fixture);
    let deadline = lost.validity.as_ref().unwrap().deadline();
    fixture.state.disconnected();
    fixture.state.control_generation = Some(uuid::Uuid::new_v4());
    crate::client_capability_support::install(&mut fixture.state);
    crate::finalization_actions::flush(
        &mut fixture.state,
        &fixture.jobs,
        std::time::Instant::now(),
    );
    assert!(fixture.state.creations.finalizing[&token(&lost)].matches_attempt(&lost));
    no_commands(&mut fixture);
    crate::finalization_actions::flush(&mut fixture.state, &fixture.jobs, deadline);
    no_commands(&mut fixture);
    let current = retry(&mut fixture, &lost);
    assert_ne!(current.generation, lost.generation);
    assert_eq!(current.command, lost.command);
    finish(&mut fixture, lost, Ok(Reply::Completed {}));
    assert!(fixture.state.creations.finalizing.is_empty());
    assert!(!current.validity.as_ref().unwrap().is_current());
    no_commands(&mut fixture);
}

#[test]
fn old_attempt_failures_cannot_requeue_new_attempt_in_the_same_generation() {
    let mut fixture = Fixture::new();
    let (_, _, first) = confirmed(&mut fixture);
    finish(
        &mut fixture,
        first.clone(),
        Err(ControlError::OperationLimit),
    );
    let current = retry(&mut fixture, &first);
    finish(&mut fixture, first.clone(), Err(ControlError::NotConnected));
    events::apply(
        &mut fixture.state,
        &fixture.jobs,
        Update::NotSent {
            request: first.clone(),
        },
    );
    let entry = &fixture.state.creations.finalizing[&token(&current)];
    assert!(entry.matches_attempt(&current));
    assert!(!entry.matches_attempt(&first));
    crate::finalization_actions::flush(&mut fixture.state, &fixture.jobs, later());
    no_commands(&mut fixture);
    let layout = fixture.state.tab_layout().clone();
    let selected = fixture.state.selected;
    let generation = fixture.state.control_generation;
    // An exact older success settles only the receipt and retires the queued retry.
    finish(&mut fixture, first, Ok(Reply::Completed {}));
    assert!(fixture.state.creations.finalizing.is_empty());
    assert!(!current.validity.as_ref().unwrap().is_current());
    assert_eq!(*fixture.state.tab_layout(), layout);
    assert_eq!(fixture.state.selected, selected);
    assert_eq!(fixture.state.control_generation, generation);
    assert!(fixture.state.writers.is_empty());
    no_commands(&mut fixture);
}

#[test]
fn unrelated_receipts_cannot_remove_or_retry_pending_finalization() {
    let mutations: &[fn(&mut yolo_hub_client_channels::ControlRequest)] = &[
        |request| request.destination = yolo_hub_identity::HubId::new(),
        |request| request.generation = None,
        |request| request.validity = None,
        |request| {
            let Command::Finalize { creation_token, .. } = &mut request.command else {
                panic!()
            };
            *creation_token = uuid::Uuid::new_v4();
        },
        |request| {
            let Command::Finalize { creation, .. } = &mut request.command else {
                panic!()
            };
            creation.creation_capability = yolo_hub_terminal_protocol::SecretCapability::generate();
        },
        |request| {
            let Command::Finalize { creation, .. } = &mut request.command else {
                panic!()
            };
            creation.address.terminal_id = yolo_hub_identity::TerminalId::new();
        },
    ];
    for mutate in mutations {
        let mut fixture = Fixture::new();
        let (_, _, original) = confirmed(&mut fixture);
        let mut unrelated = original.clone();
        mutate(&mut unrelated);
        finish(&mut fixture, unrelated.clone(), Ok(Reply::Completed {}));
        finish(
            &mut fixture,
            unrelated.clone(),
            Err(ControlError::OperationLimit),
        );
        events::apply(
            &mut fixture.state,
            &fixture.jobs,
            Update::NotSent { request: unrelated },
        );
        assert!(fixture.state.creations.finalizing[&token(&original)].matches_attempt(&original));
        no_commands(&mut fixture);
        finish(&mut fixture, original, Ok(Reply::Completed {}));
        assert!(fixture.state.creations.finalizing.is_empty());
    }
}

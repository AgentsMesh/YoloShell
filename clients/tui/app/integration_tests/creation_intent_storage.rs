use super::{
    creation_recovery_support::*,
    creation_support::{Fixture, create},
    finalization_support,
};
use crate::{creation_actions, finalization_persistence};
use std::time::Instant;
use yolo_hub_terminal_protocol::{CreateShellRequest, CreationQueryOutcome, SecretCapability};
use yolo_tui_creation_journal::CreationIntent;
use yolo_tui_tab::TabStore;

#[test]
fn failed_intent_preparation_never_queues_create_and_recovery_clears_only_live_unsent_evidence() {
    let root = tempfile::tempdir().unwrap();
    let mut fixture = Fixture::new();
    let store = TabStore::open(root.path(), fixture.origin).unwrap();
    finalization_persistence::open(&mut fixture.state, &store, fixture.origin).unwrap();
    let (first, _, _) = finalization_support::confirmed(&mut fixture);
    let journal = fixture.state.creations.journal.as_mut().unwrap();
    let mut conflicting = journal.pending().unwrap().remove(0);
    conflicting.creation.creation_capability = SecretCapability::generate();
    assert!(journal.retain_confirmed(conflicting).is_err());
    let mut fields = create(&first).fields().clone();
    fields.request_id = uuid::Uuid::new_v4();
    fields.creation_token = uuid::Uuid::new_v4();
    let request = CreateShellRequest::new(fields).unwrap();
    assert!(!creation_actions::enqueue(
        &mut fixture.state,
        &fixture.jobs,
        fixture.origin,
        request.clone()
    ));
    assert!(fixture.incoming.try_recv().is_err());
    assert!(fixture.state.creations.pending.is_empty());
    assert_eq!(
        fixture.state.creations.uncreated[&request.fields().creation_token].request,
        request
    );
    assert!(fixture.state.creations.persistence_error.is_some());
    assert!(!creation_actions::enqueue(
        &mut fixture.state,
        &fixture.jobs,
        fixture.origin,
        request.clone()
    ));
    assert!(fixture.incoming.try_recv().is_err());

    finalization_persistence::request_recovery(&mut fixture.state, Instant::now());
    assert!(fixture.state.creations.uncreated.is_empty());
    assert!(fixture.state.creations.persistence_error.is_none());
    let journal = fixture.state.creations.journal.as_ref().unwrap();
    assert!(journal.intents().unwrap().is_empty());
    assert_eq!(journal.pending().unwrap().len(), ONE_RECORD);
    assert!(fixture.incoming.try_recv().is_err());
    assert!(creation_actions::enqueue(
        &mut fixture.state,
        &fixture.jobs,
        fixture.origin,
        request
    ));
    assert!(matches!(
        fixture.incoming.try_recv().unwrap(),
        crate::network::Job::Call {
            command: yolo_hub_rpc_protocol::Command::Create { .. },
            ..
        }
    ));
}

#[test]
fn an_already_durable_intent_never_authorizes_an_initial_create_again() {
    let root = tempfile::tempdir().unwrap();
    let mut fixture = Fixture::new();
    let store = TabStore::open(root.path(), fixture.origin).unwrap();
    finalization_persistence::open(&mut fixture.state, &store, fixture.origin).unwrap();
    let request = fixture.start();
    // Simulate loss of the transient UI owner, retaining the durable transaction.
    fixture.state.creations.pending.clear();
    assert!(!creation_actions::enqueue(
        &mut fixture.state,
        &fixture.jobs,
        fixture.origin,
        create(&request).clone()
    ));
    assert!(fixture.incoming.try_recv().is_err());
    assert_eq!(fixture.state.creations.recovery.len(), ONE_RECORD);
    let query = take_query(&mut fixture, Instant::now());
    assert!(matches!(
        query.command,
        yolo_hub_rpc_protocol::Command::QueryCreation { .. }
    ));
}

#[test]
fn recovered_ready_result_remains_in_memory_until_exact_journal_recovery_can_promote_it() {
    let root = tempfile::tempdir().unwrap();
    let mut fixture = Fixture::new();
    let store = TabStore::open(root.path(), fixture.origin).unwrap();
    finalization_persistence::open(&mut fixture.state, &store, fixture.origin).unwrap();
    let request = fixture.start();
    retire(&mut fixture, request.clone());
    let query = take_query(&mut fixture, Instant::now());
    let mut fields = create(&request).fields().clone();
    fields.name = Some(CHANGED_NAME.into());
    let conflicting =
        CreationIntent::new(fixture.origin, CreateShellRequest::new(fields).unwrap()).unwrap();
    assert!(
        fixture
            .state
            .creations
            .journal
            .as_mut()
            .unwrap()
            .retain_intent(conflicting)
            .is_err()
    );
    let created = fixture.created();
    let before = fixture.state.tab_snapshot();
    reply(
        &mut fixture,
        query.clone(),
        CreationQueryOutcome::Ready { creation: created },
    );
    assert_eq!(
        fixture.state.creations.recovery.status().resolved,
        ONE_RECORD
    );
    assert!(fixture.state.creations.persistence_error.is_some());
    assert!(fixture.state.creations.finalizing.is_empty());
    finalization_support::no_commands(&mut fixture);
    // An older weak result cannot erase the retained Ready outcome.
    reply(&mut fixture, query, CreationQueryOutcome::Missing {});
    assert_eq!(
        fixture.state.creations.recovery.status().resolved,
        ONE_RECORD
    );
    let at = fixture.state.creations.persistence_recover_at.unwrap();
    crate::polling::renew_leases(&mut fixture.state, &fixture.jobs);
    finalization_persistence::recover_due(&mut fixture.state, at);
    crate::creation_recovery::flush(&mut fixture.state, &fixture.jobs, at);
    crate::finalization_actions::flush(&mut fixture.state, &fixture.jobs, at);
    let finalized = finalization_support::take(&mut fixture);
    assert!(fixture.state.creations.recovery.is_empty());
    assert!(
        fixture
            .state
            .creations
            .journal
            .as_ref()
            .unwrap()
            .intents()
            .unwrap()
            .is_empty()
    );
    assert_eq!(fixture.state.tab_snapshot(), before);
    assert!(fixture.state.writers.is_empty());
    assert!(
        matches!(finalized.command, yolo_hub_rpc_protocol::Command::Finalize { creation, .. } if creation == created)
    );
}

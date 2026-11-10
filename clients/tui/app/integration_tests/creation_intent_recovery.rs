use super::{
    creation_recovery_support::*,
    creation_support::{Fixture, create},
    finalization_support,
};
use crate::finalization_persistence;
use std::time::Instant;
use yolo_hub_rpc_protocol::{Command, Reply};
use yolo_hub_terminal_protocol::CreationQueryOutcome;
use yolo_tui_tab::TabStore;

#[test]
fn durable_intent_precedes_create_and_restart_queries_without_restoring_view_authority() {
    let root = tempfile::tempdir().unwrap();
    let mut first = Fixture::new();
    let hub = first.origin;
    let store = TabStore::open(root.path(), hub).unwrap();
    finalization_persistence::open(&mut first.state, &store, hub).unwrap();
    let request = first.start();
    let journal = first.state.creations.journal.as_ref().unwrap();
    let intents = journal.intents().unwrap();
    assert_eq!(intents.len(), ONE_RECORD);
    assert_eq!(intents[0].request, *create(&request));
    assert_eq!(intents[0].origin, request.destination);
    assert!(journal.pending().unwrap().is_empty());
    drop(first);
    drop(store);

    let store = TabStore::open(root.path(), hub).unwrap();
    let mut restored = Fixture::new();
    restored.origin = hub;
    restored.state = crate::tab_persistence::restore(&store);
    finalization_persistence::open(&mut restored.state, &store, hub).unwrap();
    let tab_catalog = restored.state.tab_snapshot();
    assert!(restored.state.creations.uncreated.is_empty());
    assert!(restored.state.creations.pending.is_empty());
    assert_eq!(restored.state.creations.recovery.len(), ONE_RECORD);
    ready(&mut restored);
    let query = take_query(&mut restored, Instant::now());
    let Command::QueryCreation { request: queried } = &query.command else {
        panic!()
    };
    assert_eq!(queried.creation(), create(&request));
    assert_eq!(query.destination, request.destination);
    let creation = restored.created();
    reply(
        &mut restored,
        query,
        CreationQueryOutcome::Ready { creation },
    );
    let finalization = finalization_support::take(&mut restored);
    let journal = restored.state.creations.journal.as_ref().unwrap();
    assert!(journal.intents().unwrap().is_empty());
    assert_eq!(journal.pending().unwrap().len(), ONE_RECORD);
    assert!(restored.state.creations.recovery.is_empty());
    assert_eq!(restored.state.tab_snapshot(), tab_catalog);
    assert!(restored.state.writers.is_empty());
    assert!(restored.state.selected.is_none());
    finalization_support::finish(&mut restored, finalization, Ok(Reply::Completed {}));
    drop(restored);
    drop(store);
    let store = TabStore::open(root.path(), hub).unwrap();
    let mut state = crate::tab_persistence::restore(&store);
    finalization_persistence::open(&mut state, &store, hub).unwrap();
    assert_eq!(state.creations.len(), usize::default());
}

#[test]
fn missing_pending_unknown_never_remove_intent_or_authorize_another_create() {
    for outcome in [
        CreationQueryOutcome::Missing {},
        CreationQueryOutcome::Pending {},
        CreationQueryOutcome::Unknown {},
    ] {
        let root = tempfile::tempdir().unwrap();
        let mut fixture = Fixture::new();
        let store = TabStore::open(root.path(), fixture.origin).unwrap();
        finalization_persistence::open(&mut fixture.state, &store, fixture.origin).unwrap();
        let request = fixture.start();
        retire(&mut fixture, request.clone());
        let query = take_query(&mut fixture, Instant::now());
        reply(&mut fixture, query, outcome);
        let journal = fixture.state.creations.journal.as_ref().unwrap();
        assert_eq!(journal.intents().unwrap().len(), ONE_RECORD);
        assert!(journal.pending().unwrap().is_empty());
        assert!(!crate::creation_actions::enqueue(
            &mut fixture.state,
            &fixture.jobs,
            request.destination,
            create(&request).clone()
        ));
        assert!(fixture.state.creations.pending.is_empty());
        assert!(fixture.state.creations.finalizing.is_empty());
        finalization_support::no_commands(&mut fixture);
    }
}

#[test]
fn terminal_query_outcomes_settle_only_the_transaction_without_fabricating_a_live_session() {
    for kind in [
        Completion::Finalized,
        Completion::Survivor,
        Completion::Archived,
    ] {
        let root = tempfile::tempdir().unwrap();
        let mut fixture = Fixture::new();
        let store = TabStore::open(root.path(), fixture.origin).unwrap();
        finalization_persistence::open(&mut fixture.state, &store, fixture.origin).unwrap();
        let request = fixture.start();
        retire(&mut fixture, request);
        let query = take_query(&mut fixture, Instant::now());
        let creation = fixture.created();
        let outcome = match kind {
            Completion::Finalized => CreationQueryOutcome::Finalized { creation },
            Completion::Survivor => CreationQueryOutcome::Survivor { creation },
            Completion::Archived => CreationQueryOutcome::Archived {
                address: creation.address,
            },
        };
        let before = fixture.state.tab_snapshot();
        reply(&mut fixture, query, outcome);
        let journal = fixture.state.creations.journal.as_ref().unwrap();
        assert!(journal.intents().unwrap().is_empty());
        assert!(journal.pending().unwrap().is_empty());
        assert_eq!(fixture.state.tab_snapshot(), before);
        assert!(!fixture.state.terminals.contains_key(&creation.address));
        assert!(fixture.state.writers.is_empty());
        assert!(fixture.state.creations.finalizing.is_empty());
        finalization_support::no_commands(&mut fixture);
    }
}
enum Completion {
    Finalized,
    Survivor,
    Archived,
}

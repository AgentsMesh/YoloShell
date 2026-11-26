use super::{
    creation_support::{Fixture, create},
    finalization_support::*,
};
use crate::{events, finalization_persistence, network::Update};
use yolo_hub_client_channels::ControlError;
use yolo_hub_rpc_protocol::Reply;
use yolo_tui_tab::TabStore;

const DISPLACED_JOURNAL: &str = "retained-original-journal.json";

#[test]
fn failed_retention_preserves_the_first_known_result_until_exact_store_recovery() {
    let root = tempfile::tempdir().unwrap();
    let mut fixture = Fixture::new();
    let store = TabStore::open(root.path(), fixture.origin).unwrap();
    finalization_persistence::open(&mut fixture.state, &store, fixture.origin).unwrap();
    let (_, original, _) = confirmed(&mut fixture);
    // The new request is durably prepared before the later result-save failure.
    let request = fixture.start();
    let journal = fixture.state.creations.journal.as_mut().unwrap();
    let mut conflicting = journal.pending().unwrap().remove(0);
    conflicting.creation.creation_capability =
        yolo_hub_terminal_protocol::SecretCapability::generate();
    assert!(journal.retain_confirmed(conflicting).is_err());
    let created = fixture.created();
    fixture.finish(request.clone(), created);
    let id = create(&request).fields().request_id;
    assert_eq!(
        fixture.state.creations.pending[&id].confirmed,
        Some(created)
    );
    assert_eq!(fixture.original_binding(), Some(original.address));
    assert!(fixture.state.creations.persistence_error.is_some());
    let different = fixture.created();
    fixture.finish(request.clone(), different);
    finish(
        &mut fixture,
        request.clone(),
        Err(ControlError::NotConnected),
    );
    events::apply(
        &mut fixture.state,
        &fixture.jobs,
        Update::NotSent {
            request: request.clone(),
        },
    );
    assert_eq!(
        fixture.state.creations.pending[&id].confirmed,
        Some(created)
    );
    no_commands(&mut fixture);
    let now = fixture.state.creations.persistence_recover_at.unwrap();
    finalization_persistence::recover_due(&mut fixture.state, now);
    crate::creation_confirmation::flush(&mut fixture.state, &fixture.jobs, now);
    crate::finalization_actions::flush(&mut fixture.state, &fixture.jobs, now);
    let finalized = take(&mut fixture);
    assert!(fixture.state.creations.pending.is_empty());
    assert!(fixture.state.creations.persistence_error.is_none());
    assert_eq!(fixture.original_binding(), Some(created.address));
    assert_eq!(
        fixture
            .state
            .creations
            .journal
            .as_ref()
            .unwrap()
            .pending()
            .unwrap()
            .len(),
        2
    );
    finish(&mut fixture, finalized, Ok(Reply::Completed {}));
    assert_eq!(
        fixture
            .state
            .creations
            .journal
            .as_ref()
            .unwrap()
            .pending()
            .unwrap()
            .len(),
        1
    );
}

#[test]
fn failed_acknowledgement_keeps_evidence_and_stops_retrying_a_replaced_journal() {
    let root = tempfile::tempdir().unwrap();
    let mut fixture = Fixture::new();
    let store = TabStore::open(root.path(), fixture.origin).unwrap();
    finalization_persistence::open(&mut fixture.state, &store, fixture.origin).unwrap();
    let (_, _, request) = confirmed(&mut fixture);
    let journal = root.path().join(yolo_tui_creation_journal::JOURNAL_FILE);
    let displaced = root.path().join(DISPLACED_JOURNAL);
    let retained_bytes = std::fs::read(&journal).unwrap();
    std::fs::rename(&journal, &displaced).unwrap();
    finish(&mut fixture, request.clone(), Ok(Reply::Completed {}));
    assert!(fixture.state.creations.finalizing[&token(&request)].acknowledged);
    assert!(fixture.state.creations.persistence_error.is_some());
    no_commands(&mut fixture);
    let recovery_at = fixture.state.creations.persistence_recover_at;
    let layout = fixture.state.tab_layout().clone();
    finish(
        &mut fixture,
        request.clone(),
        Err(ControlError::OperationLimit),
    );
    events::apply(
        &mut fixture.state,
        &fixture.jobs,
        Update::NotSent {
            request: request.clone(),
        },
    );
    let retained = &fixture.state.creations.finalizing[&token(&request)];
    assert!(retained.acknowledged);
    assert!(retained.attempt.is_none());
    assert!(retained.rejection.is_none());
    assert_eq!(fixture.state.creations.persistence_recover_at, recovery_at);
    assert_eq!(*fixture.state.tab_layout(), layout);
    no_commands(&mut fixture);
    let retry_at = fixture.state.creations.persistence_recover_at.unwrap();
    finalization_persistence::recover_due(&mut fixture.state, retry_at);
    assert!(fixture.state.creations.persistence_error.is_some());
    assert!(fixture.state.creations.persistence_recover_at.is_none());
    crate::terminal_actions::prefix(
        &mut fixture.state,
        &fixture.jobs,
        fixture.origin,
        crossterm::event::KeyCode::Char(finalization_persistence::RECOVER_STORAGE_KEY),
    );
    crate::finalization_actions::flush(&mut fixture.state, &fixture.jobs, later());
    assert_eq!(fixture.state.creations.finalizing.len(), 1);
    assert!(!journal.exists());
    assert_eq!(std::fs::read(displaced).unwrap(), retained_bytes);
    no_commands(&mut fixture);
    assert!(
        finalization_persistence::status(&fixture.state)
            .unwrap()
            .contains("Ctrl+B f")
    );
}

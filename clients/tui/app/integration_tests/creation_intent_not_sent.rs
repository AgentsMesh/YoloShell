use super::{
    creation_recovery_support::*,
    creation_support::{Fixture, create},
    finalization_support,
};
use crate::{events, finalization_persistence, network::Update};
use std::time::Instant;
use yolo_hub_client_channels::{ControlError, ControlEvent};
use yolo_hub_terminal_protocol::{CreateShellRequest, PublicError};
use yolo_tui_creation_journal::CreationIntent;
use yolo_tui_tab::TabStore;

#[test]
fn correlated_no_resource_outcomes_release_capacity_without_querying_or_recreating() {
    for failure in [
        ControlError::NotConnected,
        ControlError::StaleGeneration,
        ControlError::Invalidated,
        ControlError::OperationLimit,
        ControlError::Client(yolo_hub_client::ClientError::Remote(
            PublicError::InvalidDirectory,
        )),
    ] {
        let root = tempfile::tempdir().unwrap();
        let mut fixture = Fixture::new();
        let store = TabStore::open(root.path(), fixture.origin).unwrap();
        finalization_persistence::open(&mut fixture.state, &store, fixture.origin).unwrap();
        let request = fixture.start();
        events::apply(
            &mut fixture.state,
            &fixture.jobs,
            Update::Control(ControlEvent::Finished {
                request: request.clone(),
                result: Err(failure),
                connected: true,
            }),
        );
        assert_eq!(fixture.state.creations.len(), usize::default());
        let journal = fixture.state.creations.journal.as_ref().unwrap();
        assert!(journal.intents().unwrap().is_empty());
        assert!(journal.pending().unwrap().is_empty());
        finalization_support::no_commands(&mut fixture);
        let creation = fixture.created();
        fixture.finish(request, creation);
        assert_eq!(fixture.original_binding(), Some(fixture.expected));
        assert!(fixture.state.writers.is_empty());
        finalization_support::no_commands(&mut fixture);
    }
}

#[test]
fn multiple_unsent_attempts_keep_live_evidence_until_the_original_store_recovers() {
    let root = tempfile::tempdir().unwrap();
    let mut fixture = Fixture::new();
    let store = TabStore::open(root.path(), fixture.origin).unwrap();
    finalization_persistence::open(&mut fixture.state, &store, fixture.origin).unwrap();
    let first = fixture.start();
    let second = fixture.start();
    let mut fields = create(&first).fields().clone();
    fields.name = Some(CHANGED_NAME.into());
    assert!(
        fixture
            .state
            .creations
            .journal
            .as_mut()
            .unwrap()
            .retain_intent(
                CreationIntent::new(fixture.origin, CreateShellRequest::new(fields).unwrap())
                    .unwrap()
            )
            .is_err()
    );
    for request in [&first, &second] {
        events::apply(
            &mut fixture.state,
            &fixture.jobs,
            Update::NotSent {
                request: request.clone(),
            },
        );
        assert!(
            fixture
                .state
                .creations
                .uncreated
                .contains_key(&create(request).fields().creation_token)
        );
    }
    assert!(fixture.state.creations.pending.is_empty());
    assert!(fixture.state.creations.recovery.is_empty());
    assert!(fixture.state.creations.persistence_error.is_some());
    finalization_support::no_commands(&mut fixture);
    finalization_persistence::request_recovery(&mut fixture.state, Instant::now());
    assert!(fixture.state.creations.uncreated.is_empty());
    assert!(fixture.state.creations.persistence_error.is_none());
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
    finalization_support::no_commands(&mut fixture);
}

use super::{creation_support::Fixture, finalization_support::*};
use crate::{events, finalization_persistence, network::Update};
use yolo_hub_client::ClientError;
use yolo_hub_client_channels::ControlError;
use yolo_hub_rpc_protocol::Reply;
use yolo_hub_terminal_protocol::PublicError;
use yolo_tui_tab::TabStore;

#[test]
fn confirmed_receipt_reopens_as_only_finalize_and_acknowledgement_is_durable() {
    let root = tempfile::tempdir().unwrap();
    let mut original = Fixture::new();
    let hub = original.origin;
    let store = TabStore::open(root.path(), hub).unwrap();
    finalization_persistence::open(&mut original.state, &store, hub).unwrap();
    let (_, _, admitted) = confirmed(&mut original);
    let records = original
        .state
        .creations
        .journal
        .as_ref()
        .unwrap()
        .pending()
        .unwrap();
    assert_eq!(records.len(), 1);
    drop(original);
    drop(store);
    let store = TabStore::open(root.path(), hub).unwrap();
    let mut restored = Fixture::new();
    restored.state = crate::tab_persistence::restore(&store);
    let tab_catalog = restored.state.tab_snapshot();
    finalization_persistence::open(&mut restored.state, &store, hub).unwrap();
    assert_eq!(restored.state.creations.finalizing.len(), 1);
    assert!(restored.state.creations.pending.is_empty());
    assert!(restored.state.writers.is_empty());
    assert!(restored.state.selected.is_none());
    assert!(restored.state.control_generation.is_none());
    events::apply(
        &mut restored.state,
        &restored.jobs,
        Update::Control(crate::client_capability_support::ready(uuid::Uuid::new_v4())),
    );
    let retry = take(&mut restored);
    assert_eq!(retry.command, admitted.command);
    assert_ne!(retry.generation, admitted.generation);
    assert_eq!(restored.state.tab_snapshot(), tab_catalog);
    // A restored rejection must be visible without pretending it belongs to a pane.
    finish(
        &mut restored,
        retry.clone(),
        Err(ControlError::Client(ClientError::Remote(
            PublicError::Unauthorized,
        ))),
    );
    assert_eq!(crate::finalization_status::blocked(&restored.state), 1);
    let detail = crate::finalization_status::detail(&restored.state).unwrap();
    assert!(detail.contains(&hub.to_string()));
    assert!(
        detail.contains(
            &records[0]
                .creation
                .address
                .terminal_id
                .as_uuid()
                .to_string()
        )
    );
    assert!(!detail.contains(&records[0].request.fields().creation_token.to_string()));
    assert_eq!(restored.state.status, crate::constants::CONNECTED_LABEL);
    finish(&mut restored, retry, Ok(Reply::Completed {}));
    assert!(restored.state.creations.finalizing.is_empty());
    assert!(
        restored
            .state
            .creations
            .journal
            .as_ref()
            .unwrap()
            .pending()
            .unwrap()
            .is_empty()
    );
    assert_eq!(restored.state.tab_snapshot(), tab_catalog);
    assert!(restored.state.writers.is_empty());
    drop(restored);
    drop(store);
    let store = TabStore::open(root.path(), hub).unwrap();
    let mut state = crate::tab_persistence::restore(&store);
    finalization_persistence::open(&mut state, &store, hub).unwrap();
    assert!(state.creations.finalizing.is_empty());
}

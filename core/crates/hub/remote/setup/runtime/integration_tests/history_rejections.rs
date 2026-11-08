use super::{
    constants::*,
    fixture::{Fixture, call},
};
use std::sync::Arc;
use yolo_hub_dispatch::Dispatcher;
use yolo_hub_identity::HubId;
use yolo_hub_rpc_protocol::{Command, Request};
use yolo_hub_terminal_protocol::PublicError;

#[tokio::test]
async fn history_cannot_target_a_different_entry_hub() {
    let fixture = Fixture::new(false, false).await;
    let request = Request::new(
        HubId::new(),
        Command::InspectRemoteSetup {
            operation_id: fixture.request.request.fields().request_id,
        },
    )
    .unwrap();
    assert!(matches!(
        fixture.runtime.dispatch(request).await,
        Err(PublicError::Unauthorized)
    ));
    assert!(fixture.broker.state.lock().unwrap().calls.is_empty());
    fixture.finish().await;
}

#[tokio::test]
async fn unsupported_management_layer_does_not_misinterpret_history_as_configuration_read() {
    let fixture = Fixture::new(false, false).await;
    assert_eq!(
        call(
            &(fixture.managed.clone() as Arc<dyn Dispatcher>),
            Command::InspectRemoteSetup {
                operation_id: fixture.request.request.fields().request_id
            }
        )
        .await
        .unwrap_err(),
        PublicError::Unsupported
    );
    fixture.finish().await;
}

#[tokio::test]
async fn replaced_operation_directory_cannot_be_read_as_legitimate_history() {
    let fixture = Fixture::new(false, false).await;
    fixture.seed_prepared().await;
    let entry = fixture.record().parent().unwrap().to_path_buf();
    let saved = fixture.root.path().join(SAVED_RECORD);
    std::fs::rename(&entry, &saved).unwrap();
    yolo_paths::directory::PrivateDirectory::ensure(&entry).unwrap();
    std::fs::copy(saved.join(RECORD_FILE), fixture.record()).unwrap();
    let result = fixture
        .client()
        .await
        .inspect_remote_setup(fixture.request.request.fields().request_id, deadline())
        .await;
    assert!(matches!(
        result,
        Err(yolo_hub_client::ClientError::Remote(
            PublicError::Unreachable
        ))
    ));
    assert!(fixture.broker.state.lock().unwrap().calls.is_empty());
    fixture.finish().await;
}

#[tokio::test]
async fn malformed_or_missing_record_is_an_error_not_permission_to_restart_setup() {
    let fixture = Fixture::new(false, false).await;
    fixture.seed_prepared().await;
    std::fs::write(fixture.record(), b"invalid setup journal").unwrap();
    let result = fixture
        .client()
        .await
        .inspect_remote_setup(fixture.request.request.fields().request_id, deadline())
        .await;
    assert!(matches!(
        result,
        Err(yolo_hub_client::ClientError::Remote(
            PublicError::Unreachable
        ))
    ));
    std::fs::rename(fixture.record(), fixture.root.path().join(SAVED_RECORD)).unwrap();
    let result = fixture
        .client()
        .await
        .inspect_remote_setup(fixture.request.request.fields().request_id, deadline())
        .await;
    assert!(matches!(
        result,
        Err(yolo_hub_client::ClientError::Remote(
            PublicError::Unreachable
        ))
    ));
    assert!(fixture.broker.state.lock().unwrap().calls.is_empty());
    fixture.finish().await;
}

#[tokio::test]
async fn a_valid_record_owned_by_another_hub_is_not_exposed() {
    let fixture = Fixture::new(false, false).await;
    let other = yolo_hub_remote_setup_protocol_test_support::fixture::request()
        .request
        .fields()
        .bootstrap
        .fields()
        .consumer;
    fixture.seed_prepared_for(other).await;
    let result = fixture
        .client()
        .await
        .inspect_remote_setup(fixture.request.request.fields().request_id, deadline())
        .await;
    assert!(matches!(
        result,
        Err(yolo_hub_client::ClientError::Remote(
            PublicError::Unauthorized
        ))
    ));
    assert!(fixture.broker.state.lock().unwrap().calls.is_empty());
    fixture.finish().await;
}

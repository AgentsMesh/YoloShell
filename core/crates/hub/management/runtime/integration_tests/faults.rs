use super::fixture::*;
use crate::*;
use tokio::sync::watch;
use yolo_hub_dispatch::Dispatcher;
use yolo_hub_rpc_protocol::{Command, Request};

const ACTIVATION_RETRY_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(5);
const ACTIVATION_RETRY_POLL: std::time::Duration = std::time::Duration::from_millis(20);

#[tokio::test]
async fn accepted_update_finishes_after_caller_cancels_during_generation_drain() {
    let Fixture {
        directory: _directory,
        state: _state,
        hub,
        service,
    } = Fixture::new().await;
    let mut generation_stop = service.running.as_ref().unwrap().stop.subscribe();
    let initial = service.snapshot().unwrap();
    let update = change(&initial);
    let expected = update.request_id;
    let guard = hub.generation.read().await;
    let (stop, stopped) = watch::channel(false);
    let service = tokio::spawn(service.run(stopped));
    let node = hub.clone();
    let caller =
        tokio::spawn(async move { call(&node, Command::Configure { change: update }).await });
    generation_stop.changed().await.unwrap();
    caller.abort();
    assert!(caller.await.unwrap_err().is_cancelled());
    drop(guard);
    let applied = call(&hub, Command::GetConfiguration {}).await.unwrap();
    assert_eq!(applied.state.last_change, expected);
    assert!(applied.applied);
    stop.send_replace(true);
    service.await.unwrap().unwrap();
}

#[tokio::test]
async fn pending_activation_recovers_without_another_management_request() {
    let Fixture {
        directory: _directory,
        state: _state,
        hub,
        mut service,
    } = Fixture::new().await;
    service.retire().await.unwrap();
    service.activation_pending = true;
    let (stop, stopped) = watch::channel(false);
    let task = tokio::spawn(service.run(stopped));
    tokio::time::timeout(ACTIVATION_RETRY_TIMEOUT, async {
        loop {
            let snapshot = call(&hub, Command::GetConfiguration {}).await.unwrap();
            if snapshot.applied {
                break;
            }
            tokio::time::sleep(ACTIVATION_RETRY_POLL).await;
        }
    })
    .await
    .expect("activation retry did not run autonomously");
    stop.send_replace(true);
    task.await.unwrap().unwrap();
}

#[tokio::test]
async fn missing_configuration_record_disables_old_federation_without_recreation() {
    const RECORD_FILE: &str = "configuration.json";
    let mut fixture = Fixture::new().await;
    let record = fixture
        .directory
        .path()
        .join("hub")
        .join(CONFIGURATION_DIRECTORY)
        .join(RECORD_FILE);
    let initial = fixture.service.snapshot().unwrap();
    std::fs::remove_file(&record).unwrap();
    assert_eq!(
        fixture.service.change(change(&initial)).await,
        Err(PublicError::Unreachable)
    );
    assert!(fixture.service.running.is_none());
    assert!(!record.exists());
    let request = Request::new(fixture.hub.hub_id(), Command::ListHubs { after: None }).unwrap();
    assert!(matches!(
        fixture.hub.dispatch(request).await,
        Err(PublicError::Unreachable)
    ));
    assert_eq!(
        fixture.service.checked_snapshot().await,
        Err(PublicError::Unreachable)
    );
}

#[tokio::test]
async fn periodic_storage_verification_retires_service_without_a_management_request() {
    const RECORD_FILE: &str = "configuration.json";
    let Fixture {
        directory,
        state: _state,
        hub,
        service,
    } = Fixture::new().await;
    let (_stop, stopped) = watch::channel(false);
    std::fs::remove_file(
        directory
            .path()
            .join("hub")
            .join(CONFIGURATION_DIRECTORY)
            .join(RECORD_FILE),
    )
    .unwrap();
    assert!(service.run(stopped).await.is_err());
    assert!(hub.generation.read().await.is_none());
}

#[tokio::test]
async fn failed_generation_retires_the_public_dispatcher() {
    let Fixture {
        directory: _directory,
        state: _state,
        hub,
        service,
    } = Fixture::new().await;
    service.running.as_ref().unwrap().stop.send_replace(true);
    let (_stop, stopped) = watch::channel(false);
    assert!(service.run(stopped).await.is_err());
    assert!(hub.generation.read().await.is_none());
}

#[tokio::test]
async fn invalid_configuration_is_rejected_before_current_generation_is_retired() {
    let mut fixture = Fixture::new().await;
    let before = fixture.service.snapshot().unwrap();
    let mut update = change(&before);
    update
        .configuration
        .trusted_origins
        .push(fixture.state.peer_identity());
    assert_eq!(
        fixture.service.change(update).await,
        Err(PublicError::InvalidRequest)
    );
    assert_eq!(fixture.service.snapshot().unwrap(), before);
    fixture.service.retire().await.unwrap();
    assert!(!fixture.service.snapshot().unwrap().applied);
}

#[tokio::test]
async fn durable_but_unapplied_change_is_not_success_and_explicit_reactivation_keeps_revision() {
    let mut fixture = Fixture::new().await;
    let correct = fixture.service.backend.clone();
    fixture.service.backend = LocalBackend::new(
        yolo_hub_identity::HubId::new(),
        fixture.directory.path(),
        &fixture.state.socket_path(),
    )
    .unwrap();
    let initial = fixture.service.snapshot().unwrap();
    let update = change(&initial);
    assert_eq!(
        fixture.service.change(update.clone()).await,
        Err(PublicError::OutcomeUnknown)
    );
    let unapplied = fixture.service.checked_snapshot().await.unwrap();
    assert!(!unapplied.applied);
    assert_eq!(unapplied.state.last_change, update.request_id);
    assert!(unapplied.state.revision > initial.state.revision);
    assert!(fixture.hub.generation.read().await.is_none());
    fixture.service.backend = correct;
    let restored = fixture.service.change(update).await.unwrap();
    assert!(restored.applied);
    assert_eq!(restored.state, unapplied.state);
    fixture.service.retire().await.unwrap();
}


include!("activation_retry_faults.rs");

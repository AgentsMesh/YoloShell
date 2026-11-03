use super::support::*;
use std::{
    os::unix::fs::PermissionsExt,
    sync::{Arc, atomic::Ordering},
};
use tokio::time::timeout_at;
use yolo_hub_client_access_registry::AccessRegistry;
use yolo_hub_identity::HubId;
use yolo_hub_public_client_server::PublicClientServer;

#[tokio::test]
async fn entry_identity_mismatches_are_rejected_before_public_socket_publication() {
    for wrong_dispatcher in [false, true] {
        let prepared = Prepared::new();
        let mut configuration = prepared.registry.current().unwrap();
        if !wrong_dispatcher {
            configuration.entry_hub = HubId::new();
        }
        let registry = AccessRegistry::new(configuration).unwrap();
        let backend = Arc::new(Backend {
            hub: if wrong_dispatcher {
                HubId::new()
            } else {
                prepared.state.hub_id()
            },
            allowed: prepared.backend.allowed,
            hidden: prepared.backend.hidden,
            calls: Default::default(),
        });
        let socket = prepared.socket();
        assert!(!socket.exists());
        let result =
            PublicClientServer::bind(prepared.state.clone(), backend.clone(), registry).await;
        assert!(result.is_err());
        assert!(!socket.exists());
        assert_eq!(backend.calls.load(Ordering::SeqCst), COUNT_ZERO);
        assert!(prepared.state.verify().is_ok());
    }
}

#[tokio::test]
async fn registry_close_retires_whole_ingress_with_authenticated_and_pending_connections() {
    let mut fixture = Fixture::start().await;
    let first = fixture.connection().await;
    let second = fixture.connection().await;
    let first_client = probes::client(&first, fixture.state.hub_id()).await;
    let second_client = probes::client(&second, fixture.state.hub_id()).await;
    let mut pending = fixture.pending_handshake().await;
    let before = fixture.backend.calls.load(Ordering::SeqCst);
    fixture.registry.close();
    fixture.failed_service().await;
    timeout_at(deadline(), first.closed()).await.unwrap();
    timeout_at(deadline(), second.closed()).await.unwrap();
    probes::drained_close(&mut pending).await;
    assert_eq!(fixture.backend.calls.load(Ordering::SeqCst), before);
    drop((first_client, second_client));
    first.close(deadline()).await.unwrap();
    second.close(deadline()).await.unwrap();
}

#[tokio::test]
async fn invalid_private_state_stops_idle_ingress_closes_all_clients_and_withdraws_socket() {
    let mut fixture = Fixture::start().await;
    let first = fixture.connection().await;
    let second = fixture.connection().await;
    let first_client = probes::client(&first, fixture.state.hub_id()).await;
    let second_client = probes::client(&second, fixture.state.hub_id()).await;
    let mut pending = fixture.pending_handshake().await;
    let state_path = fixture.state.directory().unwrap().path().to_owned();
    let before = fixture.backend.calls.load(Ordering::SeqCst);
    std::fs::set_permissions(
        &state_path,
        std::fs::Permissions::from_mode(INVALID_DIRECTORY_MODE),
    )
    .unwrap();
    assert!(fixture.state.verify().is_err());
    fixture.failed_service().await;
    timeout_at(deadline(), first.closed()).await.unwrap();
    timeout_at(deadline(), second.closed()).await.unwrap();
    probes::drained_close(&mut pending).await;
    assert_eq!(fixture.backend.calls.load(Ordering::SeqCst), before);
    std::fs::set_permissions(
        &state_path,
        std::fs::Permissions::from_mode(PRIVATE_DIRECTORY_MODE),
    )
    .unwrap();
    drop((first_client, second_client));
    first.close(deadline()).await.unwrap();
    second.close(deadline()).await.unwrap();
}

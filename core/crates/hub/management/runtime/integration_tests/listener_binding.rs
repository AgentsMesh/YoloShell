use super::{fixture::*, listener_constants::*, listener_restart::*};
use crate::*;
use std::net::TcpListener;
use yolo_hub_listener_binding::LISTENER_BINDING_FILE;

#[tokio::test]
async fn full_hub_state_reopen_retains_dynamic_endpoint_and_exact_configure_receipt() {
    let mut fixture = Fixture::new().await;
    let mut update = change(&fixture.service.snapshot().unwrap());
    update.configuration.listen = Some(LOOPBACK.parse().unwrap());
    let applied = fixture.service.change(update.clone()).await.unwrap();
    let (directory, backend) = stopped(fixture).await;
    let mut restored = reopen(directory, backend).await.unwrap();
    assert_eq!(restored.service.snapshot().unwrap(), applied);
    assert_eq!(restored.service.change(update).await.unwrap(), applied);
    restored.service.retire().await.unwrap();
}

#[tokio::test]
async fn occupied_saved_endpoint_fails_without_reallocating_or_rewriting() {
    let mut fixture = Fixture::new().await;
    let mut update = change(&fixture.service.snapshot().unwrap());
    update.configuration.listen = Some(LOOPBACK.parse().unwrap());
    let applied = fixture.service.change(update).await.unwrap();
    let path = fixture
        .state
        .directory()
        .unwrap()
        .path()
        .join(LISTENER_BINDING_FILE);
    let bytes = std::fs::read(&path).unwrap();
    let (directory, backend) = stopped(fixture).await;
    let occupied = TcpListener::bind(applied.listen_address.unwrap()).unwrap();
    let state = Arc::new(
        HubState::open(
            &directory.path().join(HUB_DIRECTORY),
            &directory.path().join(HOST_DIRECTORY),
        )
        .unwrap(),
    );
    assert!(
        ManagedHub::open(state.clone(), backend.clone(), None)
            .await
            .is_err()
    );
    assert_eq!(std::fs::read(path).unwrap(), bytes);
    drop((state, occupied));
    let mut restored = reopen(directory, backend).await.unwrap();
    assert_eq!(restored.service.snapshot().unwrap(), applied);
    restored.service.retire().await.unwrap();
}

#[tokio::test]
async fn explicit_disabled_and_fixed_endpoints_survive_restart_without_reviving_old_binding() {
    for fixed in [false, true] {
        let mut fixture = Fixture::new().await;
        let mut update = change(&fixture.service.snapshot().unwrap());
        update.configuration.listen = Some(LOOPBACK.parse().unwrap());
        let dynamic = fixture.service.change(update).await.unwrap();
        let mut disable = change(&dynamic);
        if fixed {
            let reservation = TcpListener::bind(LOOPBACK).unwrap();
            disable.configuration.listen = Some(reservation.local_addr().unwrap());
            drop(reservation);
        } else {
            disable.configuration.listen = None;
        }
        let explicit = fixture.service.change(disable).await.unwrap();
        let occupied = TcpListener::bind(dynamic.listen_address.unwrap()).unwrap();
        let (directory, backend) = stopped(fixture).await;
        let mut restored = reopen(directory, backend).await.unwrap();
        assert_eq!(restored.service.snapshot().unwrap(), explicit);
        let mut update = change(&explicit);
        update.configuration.listen = Some(LOOPBACK.parse().unwrap());
        let fresh = restored.service.change(update).await.unwrap();
        assert_ne!(fresh.listen_address, dynamic.listen_address);
        restored.service.retire().await.unwrap();
        drop(occupied);
    }
}

#[tokio::test]
async fn replaced_binding_synchronously_retires_generation_and_closes_listener() {
    let mut fixture = Fixture::new().await;
    let mut update = change(&fixture.service.snapshot().unwrap());
    update.configuration.listen = Some(LOOPBACK.parse().unwrap());
    let applied = fixture.service.change(update).await.unwrap();
    let path = fixture
        .state
        .directory()
        .unwrap()
        .path()
        .join(LISTENER_BINDING_FILE);
    let saved = fixture.directory.path().join(SAVED_BINDING);
    std::fs::rename(&path, &saved).unwrap();
    std::fs::copy(&saved, &path).unwrap();
    assert_eq!(
        fixture.service.checked_snapshot().await,
        Err(PublicError::Unreachable)
    );
    assert!(fixture.hub.generation.read().await.is_none());
    let _available = TcpListener::bind(applied.listen_address.unwrap()).unwrap();
    assert_eq!(
        fixture.service.change(change(&applied)).await,
        Err(PublicError::Unreachable)
    );
}

#[tokio::test]
async fn watcher_retires_generation_after_binding_disappears_without_client_activity() {
    let fixture = Fixture::new().await;
    let path = fixture
        .state
        .directory()
        .unwrap()
        .path()
        .join(LISTENER_BINDING_FILE);
    std::fs::rename(path, fixture.directory.path().join(SAVED_BINDING)).unwrap();
    let (_stop, stopped) = tokio::sync::watch::channel(false);
    assert!(fixture.service.run(stopped).await.is_err());
    assert!(fixture.hub.generation.read().await.is_none());
}

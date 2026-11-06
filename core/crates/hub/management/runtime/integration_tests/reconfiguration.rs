use super::fixture::*;
use crate::*;
use tokio::sync::watch;
use yolo_hub_rpc_protocol::Command;

#[tokio::test]
async fn activation_preserves_the_hub_peer_socket_and_exact_replay_receipt() {
    let Fixture {
        directory: _directory,
        state,
        hub,
        service,
    } = Fixture::new().await;
    let (stop, stopped) = watch::channel(false);
    let task = tokio::spawn(service.run(stopped));
    let initial = call(&hub, Command::GetConfiguration {}).await.unwrap();
    let expected = state.peer_socket().unwrap();
    assert_eq!(initial.peer_socket.as_ref(), Some(&expected));
    let update = change(&initial);
    let applied = call(
        &hub,
        Command::Configure {
            change: update.clone(),
        },
    )
    .await
    .unwrap();
    assert!(applied.applied);
    assert_eq!(applied.peer_socket.as_ref(), Some(&expected));
    let replay = call(&hub, Command::Configure { change: update })
        .await
        .unwrap();
    assert_eq!(applied, replay);
    let next = call(
        &hub,
        Command::Configure {
            change: change(&applied),
        },
    )
    .await
    .unwrap();
    assert_eq!(next.peer_socket, applied.peer_socket);
    assert!(next.state.revision > applied.state.revision);
    stop.send_replace(true);
    task.await.unwrap().unwrap();
}

#[tokio::test]
async fn concurrent_changes_have_one_winner_and_stale_revision_never_overwrites() {
    let Fixture {
        directory: _directory,
        state: _state,
        hub,
        service,
    } = Fixture::new().await;
    let (stop, stopped) = watch::channel(false);
    let task = tokio::spawn(service.run(stopped));
    let initial = call(&hub, Command::GetConfiguration {}).await.unwrap();
    let left = change(&initial);
    let right = change(&initial);
    let (left, right) = tokio::join!(
        call(&hub, Command::Configure { change: left }),
        call(&hub, Command::Configure { change: right })
    );
    assert!(matches!(
        (&left, &right),
        (Ok(_), Err(PublicError::Conflict)) | (Err(PublicError::Conflict), Ok(_))
    ));
    let final_state = call(&hub, Command::GetConfiguration {}).await.unwrap();
    assert_eq!(final_state, left.or(right).unwrap());
    stop.send_replace(true);
    task.await.unwrap().unwrap();
}

#[tokio::test]
async fn persisted_configuration_survives_service_restart_and_seed_conflicts_fail_closed() {
    let mut fixture = Fixture::new().await;
    let initial = fixture.service.snapshot().unwrap();
    let applied = fixture.service.change(change(&initial)).await.unwrap();
    let backend = fixture.service.backend.clone();
    fixture.service.retire().await.unwrap();
    drop(fixture.service);
    drop(fixture.hub);
    let (hub, mut service) = ManagedHub::open(fixture.state.clone(), backend.clone(), None)
        .await
        .unwrap();
    assert_eq!(service.snapshot().unwrap(), applied);
    service.retire().await.unwrap();
    drop(service);
    drop(hub);
    let remote = HubState::open(
        &fixture.directory.path().join("different-remote"),
        &fixture.directory.path().join("different-home"),
    )
    .unwrap();
    let different = Configuration {
        trusted_origins: vec![remote.peer_identity()],
        ..Configuration::default()
    };
    assert!(
        ManagedHub::open(fixture.state, backend, Some(different))
            .await
            .is_err()
    );
}

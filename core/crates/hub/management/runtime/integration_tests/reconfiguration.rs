use super::fixture::*;
use crate::*;
use tokio::sync::watch;
use yolo_hub_rpc_protocol::Command;

#[tokio::test]
async fn activation_preserves_ephemeral_listener_and_exact_replay_receipt() {
    let Fixture {
        directory: _directory,
        state: _state,
        hub,
        service,
    } = Fixture::new().await;
    let (stop, stopped) = watch::channel(false);
    let task = tokio::spawn(service.run(stopped));
    let initial = call(&hub, Command::GetConfiguration {}).await.unwrap();
    let mut update = change(&initial);
    update.configuration.listen = Some("127.0.0.1:0".parse().unwrap());
    let applied = call(
        &hub,
        Command::Configure {
            change: update.clone(),
        },
    )
    .await
    .unwrap();
    assert!(applied.applied);
    assert_ne!(applied.listen_address.unwrap().port(), 0);
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
    assert_eq!(next.listen_address, applied.listen_address);
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
async fn occupied_new_listener_does_not_disrupt_existing_generation() {
    let mut fixture = Fixture::new().await;
    let occupied = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let initial = fixture.service.snapshot().unwrap();
    let mut update = change(&initial);
    update.configuration.listen = Some(occupied.local_addr().unwrap());
    assert_eq!(
        fixture.service.change(update).await,
        Err(PublicError::Conflict)
    );
    assert_eq!(fixture.service.snapshot().unwrap(), initial);
    assert!(fixture.hub.generation.read().await.is_some());
    fixture.service.retire().await.unwrap();
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
    let different = Configuration {
        listen: Some("127.0.0.1:0".parse().unwrap()),
        ..Configuration::default()
    };
    assert!(
        ManagedHub::open(fixture.state, backend, Some(different))
            .await
            .is_err()
    );
}

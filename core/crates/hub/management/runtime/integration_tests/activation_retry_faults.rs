#[tokio::test]
async fn durable_unapplied_change_is_retried_without_a_management_request() {
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
        fixture.service.change(update).await,
        Err(PublicError::OutcomeUnknown)
    );
    assert!(fixture.service.activation_pending);
    assert!(fixture.service.running.is_none());
    let unapplied = fixture.service.checked_snapshot().await.unwrap();
    assert!(!unapplied.applied);

    assert!(!fixture.service.retry_activation().await);
    assert!(fixture.service.activation_pending);

    fixture.service.backend = correct;
    assert!(fixture.service.retry_activation().await);
    assert!(!fixture.service.activation_pending);
    assert!(fixture.service.running.is_some());
    let applied = fixture.service.snapshot().unwrap();
    assert!(applied.applied);
    assert_eq!(applied.state, unapplied.state);
    fixture.service.retire().await.unwrap();
}

#[cfg(unix)]
#[tokio::test]
async fn store_apply_failure_after_retirement_does_not_arm_a_doomed_activation_retry() {
    use std::os::unix::fs::PermissionsExt;
    let Fixture {
        directory,
        state: _state,
        hub,
        mut service,
    } = Fixture::new().await;
    let initial = service.snapshot().unwrap();
    let update = change(&initial);
    let configuration_directory = directory
        .path()
        .join("hub")
        .join(CONFIGURATION_DIRECTORY);
    let permissions = std::fs::metadata(&configuration_directory)
        .unwrap()
        .permissions();
    let mut running_stopped = service.running.as_ref().unwrap().stop.subscribe();
    // Hold retirement behind the durable planned state. Once the old
    // generation receives its stop signal, make the record unreadable so the
    // store's own re-check inside apply fails after change()'s check and
    // planned succeeded.
    let generation = hub.generation.read().await;
    let task = tokio::spawn(async move {
        let result = service.change(update).await;
        (result, service)
    });
    tokio::time::timeout(ACTIVATION_RETRY_TIMEOUT, running_stopped.changed())
        .await
        .expect("generation did not begin retiring")
        .unwrap();
    let mut read_only = permissions.clone();
    read_only.set_mode(0o500);
    std::fs::set_permissions(&configuration_directory, read_only).unwrap();
    drop(generation);

    let (result, mut service) = task.await.unwrap();
    std::fs::set_permissions(&configuration_directory, permissions).unwrap();

    assert_eq!(result, Err(PublicError::OutcomeUnknown));
    assert!(service.running.is_none());
    assert!(service.store.is_lost());
    assert!(
        !service.activation_pending,
        "a lost store cannot honor an autonomous retry"
    );
    assert!(!service.retry_activation().await);
}

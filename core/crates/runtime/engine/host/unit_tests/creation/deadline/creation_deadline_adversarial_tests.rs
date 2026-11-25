use crate::host::tests::*;
const CREATION_DEADLINE_LOCKED_SESSION_NAME: &str = "locked-ledger-deadline";

#[tokio::test(start_paused = true)]
async fn creation_deadline_expiry_while_ledger_is_locked_cannot_reserve_or_mutate() {
    let directory = tempdir().unwrap();
    let mut host = Host::open(HostConfig::new(directory.path())).unwrap();
    host.worker_ready_timeout = CREATE_DEADLINE_TEST_TIMEOUT;
    let host = Arc::new(host);
    let mut request = interleaving_request(directory.path(), Uuid::new_v4());
    request.name = Some(CREATION_DEADLINE_LOCKED_SESSION_NAME.into());
    let ledger_path = host.paths.home().join(CREATION_LEDGER_FILE_NAME);
    let durable_before = std::fs::read(&ledger_path).ok();
    let held = lock_recover(&host.creation_capabilities);
    let creating = {
        let host = Arc::clone(&host);
        tokio::spawn(async move { host.create_session(request).await })
    };
    tokio::task::yield_now().await;
    tokio::time::advance(CREATE_DEADLINE_TEST_TIMEOUT).await;
    drop(held);

    let error = creating.await.unwrap().unwrap_err();

    assert_eq!(error.code, crate::protocol::error_code::DELIVERY_UNKNOWN);
    assert!(
        lock_recover(&host.creation_capabilities)
            .bindings
            .is_empty()
    );
    assert_eq!(std::fs::read(&ledger_path).ok(), durable_before);
    assert!(lock_recover(&host.catalog).list(false).unwrap().is_empty());
}

#[tokio::test(start_paused = true)]
async fn creation_deadline_preserves_a_conflicting_checkpoint_before_spawn() {
    let fixture = CreationCheckpointFixture::new(true).await;
    let checkpoint = fixture.checkpoint(
        crate::worker::StartupStage::CommandExec,
        crate::worker::StartupPhase::Entered,
        crate::worker::StartupStatus::Active,
        Some(checkpoint_process(CHECKPOINT_TEST_CHILD_PID)),
    );
    fixture.write(&checkpoint);
    let successor = fixture.publish_successor();
    let deadline = HostOperationDeadline::after(CREATE_DEADLINE_TEST_TIMEOUT);
    tokio::time::advance(CREATE_DEADLINE_TEST_TIMEOUT).await;

    let result = fixture
        .host
        .spawn_published_worker_before(
            &fixture.manifest(),
            &successor,
            UnstartedLaunchPolicy::Retire,
            deadline,
        )
        .await;

    assert!(result.is_err());
    assert!(
        fixture
            .host
            .paths
            .worker_launch(fixture.binding.result.terminal_id)
            .exists()
    );
    assert_eq!(fixture.read(), checkpoint);
}

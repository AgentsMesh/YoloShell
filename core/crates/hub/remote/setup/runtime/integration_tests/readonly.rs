use super::{broker::Verb, constants::*, fixture::Fixture};
use yolo_hub_remote_setup_protocol::{RemoteSetupOutcome, SetupAction};

#[tokio::test]
async fn inspect_and_poll_absence_do_not_prepare_launch_or_write_an_operation() {
    let fixture = Fixture::new(false, false).await;
    for action in [SetupAction::Inspect, SetupAction::Poll] {
        assert!(matches!(
            fixture.action(action).await.unwrap().outcome,
            RemoteSetupOutcome::Unknown {}
        ));
        assert!(!fixture.record().exists());
        assert!(fixture.broker.state.lock().unwrap().calls.is_empty());
    }
    fixture.finish().await;
}

#[tokio::test]
async fn inspecting_durable_prepared_before_broker_dispatch_never_launches_or_advances_it() {
    let fixture = Fixture::new(false, false).await;
    let (registration, reservation) = fixture
        .profiles
        .reserve_profile(&fixture.request.profile().unwrap())
        .unwrap();
    drop(reservation);
    let parent = fixture.state.directory().unwrap();
    let root = parent
        .open_descendant(
            &parent
                .path()
                .join(yolo_hub_remote_setup_store::REMOTE_SETUP_DIRECTORY),
        )
        .unwrap();
    let record = yolo_hub_remote_setup_store::SetupRecord::new(
        fixture.state.peer_identity(),
        fixture.request.clone(),
        registration,
        fixture.configuration().await,
    )
    .unwrap();
    drop(yolo_hub_remote_setup_store::SetupOperation::open(root, record).unwrap());
    let before = std::fs::read(fixture.record()).unwrap();
    fixture.action(SetupAction::Inspect).await.unwrap();
    assert!(matches!(
        fixture.settled().await.outcome,
        RemoteSetupOutcome::Unknown {}
    ));
    assert_eq!(std::fs::read(fixture.record()).unwrap(), before);
    assert!(fixture.broker.state.lock().unwrap().calls.is_empty());
    fixture.finish().await;
}

#[tokio::test]
async fn completed_inspection_rechecks_live_route_without_rewriting_journal_or_reprovisioning() {
    let fixture = Fixture::new(false, false).await;
    fixture.action(SetupAction::Start).await.unwrap();
    assert!(matches!(
        fixture.settled().await.outcome,
        RemoteSetupOutcome::Linked { .. }
    ));
    let bytes = std::fs::read(fixture.record()).unwrap();
    let mut revoked = fixture.configuration().await;
    revoked.state.configuration.peers.clear();
    let revoked = fixture.configure(revoked).await;
    fixture.action(SetupAction::Inspect).await.unwrap();
    assert!(matches!(
        fixture.settled().await.outcome,
        RemoteSetupOutcome::Conflict { .. }
    ));
    assert_eq!(std::fs::read(fixture.record()).unwrap(), bytes);
    assert_eq!(fixture.configuration().await, revoked);
    assert_eq!(fixture.broker.count(Verb::Provision), ONE);
    assert_eq!(fixture.broker.count(Verb::Advance), 0);
    assert!(fixture.broker.count(Verb::Recover) >= ONE);
    fixture.finish().await;
}

#[tokio::test]
async fn same_bytes_entry_directory_replacement_cannot_return_cached_linked_status() {
    let fixture = Fixture::new(false, false).await;
    fixture.action(SetupAction::Start).await.unwrap();
    assert!(matches!(
        fixture.settled().await.outcome,
        RemoteSetupOutcome::Linked { .. }
    ));
    let entry = fixture.record().parent().unwrap().to_path_buf();
    let saved = fixture.root.path().join(SAVED_RECORD);
    std::fs::rename(&entry, &saved).unwrap();
    yolo_paths::directory::PrivateDirectory::ensure(&entry).unwrap();
    std::fs::copy(saved.join(RECORD_FILE), fixture.record()).unwrap();
    let status = fixture.action(SetupAction::Poll).await;
    assert!(
        !matches!(status, Ok(status) if matches!(status.outcome, RemoteSetupOutcome::Linked { .. }))
    );
    assert!(fixture.action(SetupAction::Inspect).await.is_err());
    assert_eq!(fixture.broker.count(Verb::Provision), ONE);
    fixture.finish().await;
}

#[tokio::test]
async fn same_bytes_record_replacement_during_provisioning_cannot_authorize_local_route_commit() {
    let fixture = Fixture::new(false, true).await;
    let baseline = fixture.configuration().await;
    fixture.action(SetupAction::Start).await.unwrap();
    fixture.broker.wait_provision().await;
    let file = fixture.record();
    let saved = fixture.root.path().join(SAVED_RECORD);
    std::fs::rename(&file, &saved).unwrap();
    std::fs::copy(&saved, &file).unwrap();
    fixture.broker.unblock();
    assert!(matches!(
        fixture.settled().await.outcome,
        RemoteSetupOutcome::Unknown {}
    ));
    assert_eq!(fixture.configuration().await, baseline);
    assert_eq!(fixture.broker.count(Verb::Provision), ONE);
    fixture.finish().await;
}

use super::fixture::Fixture;
use yolo_hub_remote_setup_protocol::{SetupCheckpoint, plan_link};
use yolo_hub_remote_setup_store::{OpenedSetup, SetupOperation, SetupPhase};
use yolo_paths::directory::PrivateDirectory;

#[tokio::test]
async fn routing_checkpoint_does_not_commit_its_pending_configuration_change() {
    let fixture = Fixture::new(false, false).await;
    fixture.seed_prepared().await;
    let baseline = fixture.configuration().await;
    let parent = fixture.state.directory().unwrap();
    let root = parent
        .open_descendant(
            &parent
                .path()
                .join(yolo_hub_remote_setup_store::REMOTE_SETUP_DIRECTORY),
        )
        .unwrap();
    let record = SetupOperation::inspect(&root, fixture.request.request.fields().request_id)
        .unwrap()
        .unwrap();
    let OpenedSetup::Existing(mut operation) = SetupOperation::open(root, record).unwrap() else {
        panic!()
    };
    let broker_directory = PrivateDirectory::open(fixture.broker.paths.state_dir())
        .unwrap()
        .durable_identity()
        .unwrap();
    operation
        .transition(SetupPhase::Provisioning { broker_directory })
        .unwrap();
    let provision = yolo_hub_remote_setup_protocol_test_support::fixture::ready(&fixture.request);
    let change = plan_link(&baseline, &fixture.request, &provision).unwrap();
    assert!(change.is_some());
    operation
        .transition(SetupPhase::Routing {
            broker_directory,
            change,
            provision,
        })
        .unwrap();
    let bytes = std::fs::read(fixture.record()).unwrap();
    let history = fixture.history().await;
    assert_eq!(
        history.checkpoint,
        Some(SetupCheckpoint::Routing {
            origin: fixture
                .request
                .request
                .fields()
                .bootstrap
                .fields()
                .expected_peer
                .unwrap()
                .hub()
        })
    );
    assert_eq!(fixture.configuration().await, baseline);
    assert_eq!(std::fs::read(fixture.record()).unwrap(), bytes);
    assert!(fixture.broker.state.lock().unwrap().calls.is_empty());
    drop(operation);
    fixture.finish().await;
}

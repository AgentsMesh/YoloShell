use super::{broker::Verb, constants::*, fixture::Fixture};
use yolo_hub_remote_setup_protocol::{RemoteSetupCall, RemoteSetupOutcome, SetupAction};
use yolo_hub_rpc_protocol::{Command, Reply};

#[tokio::test]
async fn accepted_start_returns_running_and_detached_client_does_not_cancel_hub_job() {
    let fixture = Fixture::new(false, true).await;
    let baseline = fixture.configuration().await;
    let mut client = fixture.client().await;
    let Reply::RemoteSetup { status } = client
        .call(
            fixture.state.hub_id(),
            Command::RemoteSetup {
                call: RemoteSetupCall {
                    action: SetupAction::Start,
                    request: fixture.request.clone(),
                    credentials: None,
                },
            },
            deadline(),
        )
        .await
        .unwrap()
    else {
        panic!()
    };
    assert!(matches!(status.outcome, RemoteSetupOutcome::Running {}));
    drop(client);
    fixture.broker.wait_provision().await;
    assert!(fixture.record().is_file());
    fixture.broker.unblock();
    let completed = fixture.settled().await;
    let RemoteSetupOutcome::Linked { configuration, .. } = completed.outcome else {
        panic!("{completed:?}")
    };
    assert_eq!(
        configuration.state.revision,
        baseline.state.revision + yolo_hub_management_protocol::CONFIGURATION_REVISION_STEP
    );
    assert_eq!(
        configuration.state.last_change,
        fixture.request.request.fields().request_id
    );
    assert_eq!(fixture.configuration().await, configuration);
    assert_eq!(fixture.broker.count(Verb::Provision), ONE);
    fixture.action(SetupAction::Start).await.unwrap();
    assert!(matches!(
        fixture.settled().await.outcome,
        RemoteSetupOutcome::Linked { .. }
    ));
    assert_eq!(fixture.broker.count(Verb::Provision), ONE);
    assert_eq!(fixture.configuration().await, configuration);
    fixture.finish().await;
}

#[tokio::test]
async fn explicit_continue_advances_only_the_known_published_remote_phase() {
    let fixture = Fixture::new(true, false).await;
    assert!(fixture.action(SetupAction::Continue).await.is_err());
    assert!(!fixture.record().exists());
    assert_eq!(fixture.broker.count(Verb::Provision), 0);
    fixture.action(SetupAction::Start).await.unwrap();
    assert!(matches!(
        fixture.settled().await.outcome,
        RemoteSetupOutcome::Remote { .. }
    ));
    assert_eq!(fixture.broker.count(Verb::Advance), 0);
    fixture.action(SetupAction::Continue).await.unwrap();
    assert!(matches!(
        fixture.settled().await.outcome,
        RemoteSetupOutcome::Linked { .. }
    ));
    assert_eq!(fixture.broker.count(Verb::Provision), ONE);
    assert_eq!(fixture.broker.count(Verb::Advance), ONE);
    fixture.action(SetupAction::Continue).await.unwrap();
    assert!(matches!(
        fixture.settled().await.outcome,
        RemoteSetupOutcome::Linked { .. }
    ));
    assert_eq!(fixture.broker.count(Verb::Advance), ONE);
    fixture.finish().await;
}

use super::{
    broker::Verb,
    constants::*,
    fixture::{Fixture, call},
};
use std::sync::Arc;
use yolo_hub_dispatch::Dispatcher;
use yolo_hub_remote_setup_protocol::{RemoteSetupCall, RemoteSetupOutcome, SetupAction};
use yolo_hub_rpc_protocol::{Command, Reply};

#[tokio::test]
async fn later_request_preserves_an_unpolled_completion_without_replaying_or_regranting_it() {
    let fixture = Fixture::new(false, false).await;
    fixture.action(SetupAction::Start).await.unwrap();
    let completed = tokio::time::timeout_at(deadline(), async {
        loop {
            let configuration = fixture.configuration().await;
            // Reservation release proves A's mutation future has returned; do not Poll A yet.
            if configuration.state.last_change == fixture.request.request.fields().request_id {
                if let Ok((_, reservation)) = fixture
                    .profiles
                    .reserve_profile(&fixture.request.profile().unwrap())
                {
                    drop(reservation);
                    return configuration;
                }
            }
            tokio::time::sleep(POLL_DELAY).await;
        }
    })
    .await
    .unwrap();
    let mut next = fixture.request.clone();
    let mut fields = next.request.fields().clone();
    let mut bootstrap = fields.bootstrap.fields().clone();
    let id = uuid::Uuid::new_v4();
    fields.request_id = id;
    bootstrap.request_id = id;
    fields.bootstrap =
        yolo_remote_provisioning_protocol::RemoteBootstrapRequest::new(bootstrap).unwrap();
    next.request = yolo_remote_provisioning_protocol::ProvisionRequest::new(fields).unwrap();
    next.expected_configuration_revision = completed.state.revision;
    fixture.broker.state.lock().unwrap().blocked = true;
    let dispatcher = fixture.runtime.clone() as Arc<dyn Dispatcher>;
    let Reply::RemoteSetup { status } = call(
        &dispatcher,
        Command::RemoteSetup {
            call: RemoteSetupCall {
                action: SetupAction::Start,
                request: next,
                credentials: None,
            },
        },
    )
    .await
    .unwrap() else {
        panic!()
    };
    assert!(matches!(status.outcome, RemoteSetupOutcome::Running {}));
    fixture.broker.wait_provision().await;
    let first = fixture.action(SetupAction::Poll).await.unwrap();
    assert!(
        matches!(&first.outcome, RemoteSetupOutcome::Linked { configuration, .. } if *configuration == completed)
    );
    assert_eq!(fixture.broker.count(Verb::Provision), TWO_REQUESTS);
    let revoked = fixture.configure(completed).await;
    let first = fixture.action(SetupAction::Poll).await.unwrap();
    assert!(matches!(first.outcome, RemoteSetupOutcome::Conflict { .. }));
    assert_eq!(fixture.configuration().await, revoked);
    fixture.broker.unblock();
    tokio::time::timeout_at(deadline(), async {
        loop {
            if let Ok((_, reservation)) = fixture
                .profiles
                .reserve_profile(&fixture.request.profile().unwrap())
            {
                drop(reservation);
                break;
            }
            tokio::time::sleep(POLL_DELAY).await;
        }
    })
    .await
    .unwrap();
    assert_eq!(fixture.broker.count(Verb::Provision), TWO_REQUESTS);
    fixture.finish().await;
}

const TWO_REQUESTS: usize = 2;

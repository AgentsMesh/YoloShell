use super::{
    broker::Verb,
    constants::*,
    fixture::{Fixture, call},
};
use std::sync::Arc;
use yolo_hub_dispatch::Dispatcher;
use yolo_hub_remote_setup_protocol::{RemoteSetupCall, RemoteSetupOutcome, SetupAction};
use yolo_hub_rpc_protocol::{Command, Reply, Request};
use yolo_hub_terminal_protocol::PublicError;
use yolo_hub_ssh_profile_protocol::SshProfileChange;

#[tokio::test]
async fn same_id_changed_payload_and_foreign_consumer_are_rejected_while_job_continues() {
    let fixture = Fixture::new(false, true).await;
    fixture.action(SetupAction::Start).await.unwrap();
    fixture.broker.wait_provision().await;
    let mut changed = fixture.request.clone();
    changed.expected_configuration_revision += 1;
    let dispatcher = fixture.runtime.clone() as Arc<dyn Dispatcher>;
    let result = call(
        &dispatcher,
        Command::RemoteSetup {
            call: RemoteSetupCall {
                action: SetupAction::Start,
                request: changed,
                credentials: None,
            },
        },
    )
    .await;
    assert!(matches!(result, Err(PublicError::Conflict)));
    let mut changed = fixture.request.clone();
    let mut fields = changed.request.fields().clone();
    let mut bootstrap = fields.bootstrap.fields().clone();
    bootstrap.consumer = yolo_hub_remote_startup_test_support::peer(
        yolo_hub_remote_startup_test_support::FOREIGN_HUB,
        yolo_hub_remote_startup_test_support::FOREIGN_KEY,
    );
    fields.bootstrap =
        yolo_remote_provisioning_protocol::RemoteBootstrapRequest::new(bootstrap).unwrap();
    changed.request = yolo_remote_provisioning_protocol::ProvisionRequest::new(fields).unwrap();
    let result = call(
        &dispatcher,
        Command::RemoteSetup {
            call: RemoteSetupCall {
                action: SetupAction::Start,
                request: changed,
                credentials: None,
            },
        },
    )
    .await;
    assert!(matches!(result, Err(PublicError::Unauthorized)));
    let remote = Request::new(
        yolo_hub_identity::HubId::new(),
        Command::RemoteSetup {
            call: RemoteSetupCall {
                action: SetupAction::Inspect,
                request: fixture.request.clone(),
                credentials: None,
            },
        },
    )
    .unwrap();
    assert!(matches!(
        fixture.runtime.dispatch(remote).await,
        Err(PublicError::Unauthorized)
    ));
    fixture.broker.unblock();
    assert!(matches!(
        fixture.settled().await.outcome,
        RemoteSetupOutcome::Linked { .. }
    ));
    assert_eq!(fixture.broker.count(Verb::Provision), ONE);
    fixture.finish().await;
}

#[tokio::test]
async fn concurrent_local_configuration_change_is_never_overwritten_by_ready_remote_setup() {
    let fixture = Fixture::new(false, true).await;
    fixture.action(SetupAction::Start).await.unwrap();
    fixture.broker.wait_provision().await;
    let baseline = fixture.configuration().await;
    let changed = fixture.configure(baseline).await; // A real CAS advances revision even for the same document.
    fixture.broker.unblock();
    assert!(matches!(
        fixture.settled().await.outcome,
        RemoteSetupOutcome::Conflict { .. }
    ));
    assert_eq!(fixture.configuration().await, changed);
    assert!(changed.state.configuration.peers.is_empty());
    fixture.finish().await;
}

#[tokio::test]
async fn profile_removal_is_busy_until_the_hub_owned_setup_releases_its_reservation() {
    let fixture = Fixture::new(false, true).await;
    fixture.action(SetupAction::Start).await.unwrap();
    fixture.broker.wait_provision().await;
    let Reply::SshProfiles { state } = call(&fixture.profile_dispatch, Command::GetSshProfiles {})
        .await
        .unwrap()
    else {
        panic!()
    };
    let mut change = SshProfileChange {
        request_id: uuid::Uuid::new_v4(),
        expected_revision: state.revision,
        configuration: state.configuration,
    };
    change.configuration.profiles.clear();
    assert!(matches!(
        call(
            &fixture.profile_dispatch,
            Command::ConfigureSshProfiles {
                change: change.clone()
            }
        )
        .await,
        Err(PublicError::Conflict)
    ));
    fixture.broker.unblock();
    fixture.settled().await;
    call(
        &fixture.profile_dispatch,
        Command::ConfigureSshProfiles { change },
    )
    .await
    .unwrap();
    assert!(
        !matches!(fixture.action(SetupAction::Poll).await, Ok(status) if matches!(status.outcome, RemoteSetupOutcome::Linked { .. }))
    );
    fixture.finish().await;
}

use super::{
    constants::*,
    fixture::{Fixture, call},
};
use yolo_hub_remote_setup_protocol::{SetupAction, SetupCheckpoint};
use yolo_hub_rpc_protocol::{Command, Reply};
use yolo_hub_ssh_profile_protocol::{SshProfileChange, SshProfileConfiguration};

#[tokio::test]
async fn removed_profiles_do_not_erase_historical_checkpoints_or_trigger_reconnect() {
    let fixture = Fixture::new(false, false).await;
    fixture.action(SetupAction::Start).await.unwrap();
    fixture.settled().await;
    let history = fixture.history().await;
    assert!(matches!(
        history.checkpoint,
        Some(SetupCheckpoint::Complete { .. })
    ));
    let bytes = std::fs::read(fixture.record()).unwrap();
    let calls = fixture.broker.state.lock().unwrap().calls.clone();
    let Reply::SshProfiles { state } = call(&fixture.profile_dispatch, Command::GetSshProfiles {})
        .await
        .unwrap()
    else {
        panic!()
    };
    call(
        &fixture.profile_dispatch,
        Command::ConfigureSshProfiles {
            change: SshProfileChange {
                request_id: uuid::Uuid::new_v4(),
                expected_revision: state.revision,
                configuration: SshProfileConfiguration::default(),
            },
        },
    )
    .await
    .unwrap();
    let observed = fixture
        .client()
        .await
        .inspect_remote_setup(history.operation_id, deadline())
        .await
        .unwrap();
    assert_eq!(observed, history);
    assert_eq!(fixture.fresh_reader_history().await, history);
    assert_eq!(fixture.broker.state.lock().unwrap().calls, calls);
    assert_eq!(std::fs::read(fixture.record()).unwrap(), bytes);
    fixture.finish().await;
}

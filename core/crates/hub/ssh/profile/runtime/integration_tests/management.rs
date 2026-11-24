use super::{constants::*, fixture::*};
use std::os::unix::fs::PermissionsExt;
use std::sync::Arc;
use yolo_hub_dispatch::Operation;
use yolo_hub_identity::HubId;
use yolo_hub_rpc_protocol::{Command, Reply, Request, SSH_PROFILE_METHODS};
use yolo_hub_terminal_protocol::PublicError;
use yolo_hub_ssh_profile_runtime::SshProfileRuntime;

#[tokio::test]
async fn profiles_are_local_only_and_durable_updates_are_exact_and_idempotent() {
    let fixture = Fixture::new();
    let Reply::Hello { hello } = fixture.call(Command::Hello {}).await.unwrap() else {
        panic!()
    };
    assert!(
        SSH_PROFILE_METHODS
            .iter()
            .all(|method| hello.methods.contains(method))
    );
    let profile = fixture.profile(&fixture.root.path().join(BROKER_DIRECTORY));
    let change = fixture.apply(vec![profile.clone()]).await;
    let accepted = fixture.current().await;
    fixture
        .call(Command::ConfigureSshProfiles {
            change: change.clone(),
        })
        .await
        .unwrap();
    assert_eq!(fixture.current().await, accepted);
    let mut conflicting = change.clone();
    conflicting.request_id = uuid::Uuid::new_v4();
    assert!(matches!(
        fixture
            .call(Command::ConfigureSshProfiles {
                change: conflicting
            })
            .await,
        Err(PublicError::Conflict)
    ));
    for command in [
        Command::GetSshProfiles {},
        Command::ConfigureSshProfiles { change },
    ] {
        let request = Request::new(HubId::new(), command).unwrap();
        assert!(matches!(
            fixture.dispatcher.dispatch(request).await,
            Err(PublicError::Unauthorized)
        ));
    }
    let state = fixture.state.clone();
    drop(fixture.dispatcher);
    drop(fixture.runtime);
    let restored = SshProfileRuntime::open(state.clone(), None).unwrap();
    let dispatcher = restored.dispatcher(Arc::new(Base(state.hub_id()))).unwrap();
    let Reply::SshProfiles { state } = call(&dispatcher, Command::GetSshProfiles {}).await.unwrap()
    else {
        panic!()
    };
    assert_eq!(state, accepted);
}

#[tokio::test]
async fn invalid_requests_and_replaced_registry_never_fall_through_to_inner_dispatcher() {
    let fixture = Fixture::new();
    assert!(
        fixture
            .runtime
            .dispatcher(Arc::new(Base(HubId::new())))
            .is_err()
    );
    assert!(matches!(
        fixture
            .call(Command::List {
                include_archived: false,
                after: None
            })
            .await,
        Err(PublicError::Unsupported)
    ));
    let state = fixture.current().await;
    let mut change = yolo_hub_ssh_profiles::SshProfileChange {
        request_id: uuid::Uuid::new_v4(),
        expected_revision: state.revision,
        configuration: state.configuration,
    };
    change
        .configuration
        .profiles
        .push(fixture.profile(&fixture.root.path().join(BROKER_DIRECTORY)));
    let request = Request::new(
        fixture.state.hub_id(),
        Command::ConfigureSshProfiles { change },
    )
    .unwrap();
    let Operation::Reply(Reply::SshProfiles { .. }) =
        fixture.dispatcher.dispatch(request).await.unwrap()
    else {
        panic!()
    };
    let root = fixture
        .state
        .directory()
        .unwrap()
        .path()
        .join(yolo_hub_ssh_profiles::SSH_PROFILES_DIRECTORY);
    std::fs::rename(&root, fixture.root.path().join(REPLACED_REGISTRY)).unwrap();
    std::fs::create_dir(&root).unwrap();
    std::fs::set_permissions(&root, std::fs::Permissions::from_mode(DIRECTORY_MODE)).unwrap();
    assert!(matches!(
        fixture.call(Command::GetSshProfiles {}).await,
        Err(PublicError::Unreachable)
    ));
}

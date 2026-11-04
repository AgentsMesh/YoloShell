use super::*;
mod build;
mod invite_ledger;
mod invites;
use yolo_hub_dispatch::{Dispatcher, Operation};
use yolo_hub_identity::HubId;
use yolo_hub_management_protocol::{
    CONFIGURATION_SCHEMA, ConfigurationState, FIRST_CONFIGURATION_REVISION,
};
use yolo_hub_rpc_protocol::{Command, MANAGEMENT_METHODS, Reply, Request, Response};

fn fixture() -> (
    tempfile::TempDir,
    Arc<ManagedHub>,
    mpsc::Receiver<ManagementRequest>,
) {
    let directory = tempfile::tempdir().unwrap();
    let state = Arc::new(
        HubState::open(
            &directory.path().join("hub"),
            &directory.path().join("host"),
        )
        .unwrap(),
    );
    let (commands, receiver) = mpsc::channel(constants::MANAGEMENT_QUEUE_CAPACITY);
    let hub = Arc::new(ManagedHub {
        state,
        generation: RwLock::new(None),
        commands,
        build: None,
        dialer: Arc::new(TcpPeerDialer),
    });
    (directory, hub, receiver)
}

fn command(hub: &ManagedHub, command: Command) -> Request {
    Request::new(hub.hub_id(), command).unwrap()
}

#[tokio::test]
async fn hello_advertises_management_without_claiming_remote_configuration_access() {
    let (_directory, hub, receiver) = fixture();
    let request = command(&hub, Command::Hello {});
    let Operation::Reply(reply) = hub.dispatch(request.clone()).await.unwrap() else {
        panic!()
    };
    Response::new(&request, hub.hub_id(), reply.clone())
        .validate_for(&request)
        .unwrap();
    let Reply::Hello { hello } = reply else {
        panic!()
    };
    assert!(
        MANAGEMENT_METHODS
            .iter()
            .all(|method| hello.methods.contains(method))
    );
    assert!(receiver.is_empty());
    let remote = Request::new(HubId::new(), Command::GetConfiguration {}).unwrap();
    assert!(matches!(
        hub.dispatch(remote).await,
        Err(PublicError::Unauthorized)
    ));
}

#[tokio::test]
async fn absent_generation_rejects_ordinary_operations() {
    let (_directory, hub, _) = fixture();
    assert!(matches!(
        hub.dispatch(command(&hub, Command::ListHubs { after: None }))
            .await,
        Err(PublicError::Unreachable)
    ));
}

#[tokio::test]
async fn bounded_queue_and_closed_service_do_not_accept_mutations() {
    let (_directory, hub, receiver) = fixture();
    for _ in 0..constants::MANAGEMENT_QUEUE_CAPACITY {
        let (reply, _) = oneshot::channel();
        hub.commands
            .try_send(ManagementRequest {
                change: None,
                reply,
            })
            .unwrap();
    }
    assert!(matches!(
        hub.dispatch(command(&hub, Command::GetConfiguration {}))
            .await,
        Err(PublicError::Conflict)
    ));
    drop(receiver);
    assert!(matches!(
        hub.dispatch(command(&hub, Command::GetConfiguration {}))
            .await,
        Err(PublicError::Unreachable)
    ));
}

#[tokio::test]
async fn cancelling_a_caller_does_not_remove_accepted_configuration_change() {
    let (_directory, hub, mut receiver) = fixture();
    let change = ConfigurationChange {
        request_id: uuid::Uuid::new_v4(),
        expected_revision: FIRST_CONFIGURATION_REVISION,
        configuration: Configuration::default(),
    };
    let request = command(
        &hub,
        Command::Configure {
            change: change.clone(),
        },
    );
    let node = hub.clone();
    let caller = tokio::spawn(async move { node.dispatch(request).await.map(|_| ()) });
    let accepted = receiver.recv().await.unwrap();
    caller.abort();
    assert!(caller.await.unwrap_err().is_cancelled());
    assert_eq!(accepted.change, Some(change));
    assert!(accepted.reply.is_closed());
}

#[tokio::test]
async fn lost_accepted_reply_is_uncertain_only_for_mutations() {
    let (_directory, hub, mut receiver) = fixture();
    let commands = [
        Command::GetConfiguration {},
        Command::Configure {
            change: ConfigurationChange {
                request_id: uuid::Uuid::new_v4(),
                expected_revision: FIRST_CONFIGURATION_REVISION,
                configuration: Configuration::default(),
            },
        },
    ];
    for input in commands {
        let expected = if input.mutates() {
            PublicError::OutcomeUnknown
        } else {
            PublicError::Unreachable
        };
        let request = command(&hub, input);
        let node = hub.clone();
        let caller = tokio::spawn(async move { node.dispatch(request).await.map(|_| ()) });
        drop(receiver.recv().await.unwrap());
        assert_eq!(caller.await.unwrap(), Err(expected));
    }
}

#[tokio::test]
async fn configuration_reply_preserves_actor_result() {
    let (_directory, hub, mut receiver) = fixture();
    let request = command(&hub, Command::GetConfiguration {});
    let node = hub.clone();
    let caller = tokio::spawn(async move { node.dispatch(request).await });
    let snapshot = ConfigurationSnapshot {
        state: ConfigurationState {
            schema_version: CONFIGURATION_SCHEMA,
            identity: hub.state.peer_identity(),
            revision: FIRST_CONFIGURATION_REVISION,
            last_change: uuid::Uuid::new_v4(),
            configuration: Configuration::default(),
        },
        applied: false,
        listen_address: None,
    };
    receiver
        .recv()
        .await
        .unwrap()
        .reply
        .send(Ok(snapshot.clone()))
        .unwrap();
    let Operation::Reply(Reply::Configuration { snapshot: actual }) =
        caller.await.unwrap().unwrap()
    else {
        panic!()
    };
    assert_eq!(actual, snapshot);
}

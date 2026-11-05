use super::{
    creation_support::{ConnectionPhase, connect, pending_shell},
    public_client_support,
    support::*,
};
use crate::Harness;
use yolo_hub_client::ClientError;
use yolo_hub_rpc_protocol::{Command, Reply};
use yolo_hub_terminal_protocol::{CleanupTerminationOutcome, CreateShellResult, PublicError};

#[tokio::test(flavor = "multi_thread")]
async fn real_hub_creation_cleanup_keeps_capability_fencing_across_restart() {
    let harness = Harness::start().await;
    let setup_hold = super::local_startup_support::retain_fixture_host(&harness).await;
    let root = tempfile::tempdir_in(SOCKET_TEST_ROOT).unwrap();
    let state = root.path().canonicalize().unwrap().join(STATE_DIRECTORY);
    let hub = RunningHub::start(&harness, &state).await;
    drop(setup_hold);
    let abandoned = pending_shell(&hub, &harness, ConnectionPhase::CreateAbandoned).await;
    let retained = pending_shell(&hub, &harness, ConnectionPhase::CreateRetained).await;
    let mut client = connect(&hub, &harness, ConnectionPhase::BeforeRestart).await;
    let swapped = CreateShellResult {
        address: abandoned.creation.address,
        creation_capability: retained.creation.creation_capability,
    };
    for command in [
        Command::CleanupTerminate {
            creation: swapped,
            force: true,
        },
        Command::CleanupArchive { creation: swapped },
    ] {
        assert!(matches!(
            client.call(hub.hub, command, deadline()).await,
            Err(ClientError::Remote(PublicError::Unauthorized))
        ));
    }
    assert!(matches!(
        client
            .call(
                hub.hub,
                Command::CleanupArchive {
                    creation: abandoned.creation
                },
                deadline()
            )
            .await,
        Err(ClientError::Remote(PublicError::Conflict))
    ));
    let Reply::Terminal { terminal: session } = client
        .call(
            hub.hub,
            Command::Get {
                address: abandoned.creation.address,
            },
            deadline(),
        )
        .await
        .unwrap()
    else {
        panic!()
    };
    assert_eq!(session.fields().runtime, abandoned.session.fields().runtime);
    public_client_support::call(
        &hub.socket,
        hub.hub,
        hub.hub,
        Command::Finalize {
            creation_token: retained.token,
            creation: retained.creation,
        },
    )
    .await;
    drop(client);
    let identity = hub.hub;
    hub.stop().await;

    let restored = RunningHub::start(&harness, &state).await;
    assert_eq!(restored.hub, identity);
    let mut client = connect(&restored, &harness, ConnectionPhase::AfterRestart).await;
    for command in [
        Command::CleanupTerminate {
            creation: retained.creation,
            force: true,
        },
        Command::CleanupArchive {
            creation: retained.creation,
        },
    ] {
        assert!(matches!(
            client.call(identity, command, deadline()).await,
            Err(ClientError::Remote(PublicError::Conflict))
        ));
    }
    let terminated = public_client_support::call(
        &restored.socket,
        identity,
        identity,
        Command::CleanupTerminate {
            creation: abandoned.creation,
            force: true,
        },
    )
    .await;
    assert_eq!(
        terminated,
        Reply::CreationTerminated {
            address: abandoned.creation.address,
            outcome: CleanupTerminationOutcome::Terminated,
        }
    );
    let Reply::Terminal { terminal: session } = public_client_support::call(
        &restored.socket,
        identity,
        identity,
        Command::CleanupArchive {
            creation: abandoned.creation,
        },
    )
    .await
    else {
        panic!()
    };
    assert!(session.fields().archived);
    assert_eq!(session.fields().address, abandoned.creation.address);
    let Reply::Terminal { terminal: session } = client
        .call(
            identity,
            Command::Get {
                address: retained.creation.address,
            },
            deadline(),
        )
        .await
        .unwrap()
    else {
        panic!()
    };
    assert!(!session.fields().archived);
    assert_eq!(session.fields().runtime, retained.session.fields().runtime);
    let target = session.fields().runtime.unwrap().target;
    let Reply::Lease { grant } = client
        .call(
            identity,
            acquire(target, yolo_hub_terminal_protocol::LeaseMode::Acquire {}),
            deadline(),
        )
        .await
        .unwrap()
    else {
        panic!()
    };
    client
        .call(identity, input(grant, EXIT_COMMAND), deadline())
        .await
        .unwrap();
    drop(client);
    restored.stop().await;
}

use crate::{
    ClientError,
    fixtures::{constants::*, *},
    server::*,
};
use yolo_hub_remote_setup_protocol::{RemoteSetupHistory, SetupCheckpoint};
use yolo_hub_rpc_protocol::{Command, REMOTE_SETUP_METHODS, Reply};

#[tokio::test]
async fn history_client_uses_entry_identity_and_returns_durable_checkpoint() {
    let (mut client, mut server) = connected_with(REMOTE_SETUP_METHODS.to_vec()).await;
    let operation_id = id(REQUEST_ID);
    let history = RemoteSetupHistory::new(
        operation_id,
        hub(),
        Some(SetupCheckpoint::Complete {
            origin: other_hub(),
        }),
    )
    .unwrap();
    let expected = history.clone();
    let peer = async {
        let request = receive(&mut server).await;
        assert_eq!(request.fields().destination, hub());
        assert_eq!(
            request.fields().command,
            Command::InspectRemoteSetup { operation_id }
        );
        respond(&mut server, &request, Reply::RemoteSetupHistory { history }).await;
    };
    let (result, ()) = tokio::join!(client.inspect_remote_setup(operation_id, deadline()), peer);
    assert_eq!(result.unwrap(), expected);
    assert!(client.is_usable());
}

#[tokio::test]
async fn mismatched_history_is_invalid_and_never_an_unknown_mutation_outcome() {
    for (owner, operation_id) in [(other_hub(), id(REQUEST_ID)), (hub(), id(OTHER_REQUEST_ID))] {
        let (mut client, mut server) = connected_with(REMOTE_SETUP_METHODS.to_vec()).await;
        let peer = async {
            let request = receive(&mut server).await;
            let history = RemoteSetupHistory::new(operation_id, owner, None).unwrap();
            respond(&mut server, &request, Reply::RemoteSetupHistory { history }).await;
        };
        let (result, ()) = tokio::join!(
            client.inspect_remote_setup(id(REQUEST_ID), deadline()),
            peer
        );
        assert!(matches!(result, Err(ClientError::InvalidResponse(_))));
        assert!(!client.is_usable());
    }
}

#[tokio::test]
async fn nil_history_id_fails_before_sending_and_preserves_the_connection() {
    let (mut client, _server) = connected_with(REMOTE_SETUP_METHODS.to_vec()).await;
    assert!(matches!(
        client
            .inspect_remote_setup(uuid::Uuid::nil(), deadline())
            .await,
        Err(ClientError::Validation(_))
    ));
    assert!(client.is_usable());
}

use crate::support::*;
use std::sync::atomic::Ordering;
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    time::timeout_at,
};
use yolo_hub_client::{Client, ClientError, EntryConnector};
use yolo_hub_rpc_protocol::Command;
use yolo_hub_terminal_protocol::{
    CreateShellFields, CreateShellRequest, TERMINAL_PROTOCOL_SCHEMA_VERSION, TerminalSize,
};

#[tokio::test]
async fn outer_loss_and_policy_revocation_close_channels_and_reconnect_requires_fresh_authentication()
 {
    let fixture = Fixture::start(Service::Echo).await;
    let entry = fixture.entry();
    let hub = fixture.profile.hub.hub_id;
    let mut first = entry.open(hub, deadline()).await.unwrap();
    let outer = fixture.admitted().await;
    outer
        .disconnect(
            russh::Disconnect::ByApplication,
            DISCONNECT_REASON.into(),
            LANGUAGE.into(),
        )
        .await
        .unwrap();
    assert!(
        timeout_at(deadline(), first.read_u8())
            .await
            .unwrap()
            .is_err()
    );
    let mut second = entry.open(hub, deadline()).await.unwrap();
    second.write_all(PAYLOAD).await.unwrap();
    let mut bytes = vec![u8::default(); PAYLOAD.len()];
    timeout_at(deadline(), second.read_exact(&mut bytes))
        .await
        .unwrap()
        .unwrap();
    assert_eq!(bytes, PAYLOAD);
    assert_eq!(
        fixture.counts.connections.load(Ordering::SeqCst),
        COUNT_ONE + COUNT_ONE
    );
    fixture.registry.close();
    assert!(
        timeout_at(deadline(), second.read_u8())
            .await
            .unwrap()
            .is_err()
    );
    assert!(entry.open(hub, deadline()).await.is_err());
    entry.close(deadline()).await.unwrap();
}

#[tokio::test]
async fn lost_mutation_response_is_unknown_and_opening_another_channel_never_replays_it() {
    let fixture = Fixture::start(Service::DropMutation).await;
    let entry = fixture.entry();
    let hub = fixture.profile.hub.hub_id;
    let mut client = Client::open(entry.open(hub, deadline()).await.unwrap(), hub, deadline())
        .await
        .unwrap();
    let command = Command::Create {
        request: CreateShellRequest::new(CreateShellFields {
            schema_version: TERMINAL_PROTOCOL_SCHEMA_VERSION,
            request_id: uuid::Uuid::new_v4(),
            creation_token: uuid::Uuid::new_v4(),
            name: None,
            cwd: CREATION_CWD.into(),
            size: TerminalSize::default(),
        })
        .unwrap(),
    };
    assert!(
        client
            .call(hub, command.clone(), deadline())
            .await
            .err()
            .unwrap()
            .outcome_is_unknown()
    );
    assert!(matches!(
        client.call(hub, command, deadline()).await,
        Err(ClientError::Poisoned)
    ));
    let _second = Client::open(entry.open(hub, deadline()).await.unwrap(), hub, deadline())
        .await
        .unwrap();
    assert_eq!(fixture.counts.mutations.load(Ordering::SeqCst), COUNT_ONE);
    assert_eq!(fixture.counts.connections.load(Ordering::SeqCst), COUNT_ONE);
    assert_eq!(fixture.counts.forwards.load(Ordering::SeqCst), COUNT_ONE);
    entry.close(deadline()).await.unwrap();
}

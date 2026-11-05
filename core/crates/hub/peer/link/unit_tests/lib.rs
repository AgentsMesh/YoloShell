use crate::handshake;
use crate::LinkError;
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt, duplex};
use tokio::time::Instant;
use yolo_hub_delegation_test_support::Identity;

const TIMEOUT_SECS: u64 = 1;

fn deadline() -> Instant {
    Instant::now() + Duration::from_secs(TIMEOUT_SECS)
}

#[tokio::test]
async fn client_and_server_bind_the_expected_identities() {
    let client = Identity::new();
    let server = Identity::new();
    let (mut client_stream, mut server_stream) = duplex(64 * 1024);
    let expected_client = client.identity;
    let expected_server = server.identity;
    let server_task = tokio::spawn(async move {
        let peer = handshake::server(
            &mut server_stream,
            server.identity,
            &server,
            &[expected_client],
            deadline(),
        )
        .await
        .unwrap();
        assert_eq!(peer, expected_client);
        server_stream.write_all(b"ok").await.unwrap();
    });
    handshake::client(
        &mut client_stream,
        client.identity,
        &client,
        expected_server,
        deadline(),
    )
    .await
    .unwrap();
    let mut reply = [0; 2];
    client_stream.read_exact(&mut reply).await.unwrap();
    assert_eq!(&reply, b"ok");
    server_task.await.unwrap();
}

#[tokio::test]
async fn a_wrong_server_identity_is_rejected() {
    let client = Identity::new();
    let server = Identity::new();
    let stranger = Identity::new();
    let (mut client_stream, mut server_stream) = duplex(64 * 1024);
    let expected_client = client.identity;
    tokio::spawn(async move {
        let _ = handshake::server(
            &mut server_stream,
            server.identity,
            &server,
            &[expected_client],
            deadline(),
        )
        .await;
    });
    let error = handshake::client(
        &mut client_stream,
        client.identity,
        &client,
        stranger.identity,
        deadline(),
    )
    .await
    .unwrap_err();
    assert!(matches!(error, LinkError::Identity), "{error}");
}

#[tokio::test]
async fn a_wrong_client_identity_is_rejected() {
    let client = Identity::new();
    let server = Identity::new();
    let stranger = Identity::new();
    let (mut client_stream, mut server_stream) = duplex(64 * 1024);
    let expected_server = server.identity;
    let client_task = tokio::spawn(async move {
        let _ = handshake::client(
            &mut client_stream,
            client.identity,
            &client,
            expected_server,
            deadline(),
        )
        .await;
    });
    let error = handshake::server(
        &mut server_stream,
        server.identity,
        &server,
        &[stranger.identity],
        deadline(),
    )
    .await
    .unwrap_err();
    assert!(matches!(error, LinkError::Identity), "{error}");
    let _ = client_task.await;
}

#[tokio::test]
async fn a_mismatched_identity_fails_before_the_hello() {
    let client = Identity::new();
    let other = Identity::new();
    let (mut stream, _) = duplex(1024);
    let error = handshake::client(
        &mut stream,
        client.identity,
        &other,
        client.identity,
        deadline(),
    )
    .await
    .unwrap_err();
    assert!(matches!(error, LinkError::Identity), "{error}");
}

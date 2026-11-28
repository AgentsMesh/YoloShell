use super::super::constants::{MANAGEMENT_SOCKET_FILE, PRIVATE_FILE_MODE};
use super::super::handlers::manage;
use super::super::server_admission::BrokerAdmission;
use super::super::{BrokerClient, BrokerClientError, BrokerConnectionPool};
use super::support::{FakeFactory, HOST_ID, test_config};
use crate::local_unix::UnexpectedAncillaryData;
use crate::local_unix::test_support::{assert_reader_eof, pipe, send_right};
use crate::protocol::ProtocolError;
use std::fs;
use std::future::Future;
use std::os::fd::AsRawFd;
use std::os::unix::fs::PermissionsExt;
use std::sync::Arc;
use std::time::Duration;
use tokio::io::AsyncReadExt;
use tokio::net::{UnixListener, UnixStream};

const ANCILLARY_MARKER: u8 = b'B';
const TRANSPORT_SETTLEMENT_TIMEOUT: Duration = Duration::from_secs(2);
const TRANSPORT_SETTLEMENT_TIMEOUT_MESSAGE: &str = "management transport test did not settle";
const ANCILLARY_ERROR_EXPECTED: &str = "management transport did not report ancillary data";

#[tokio::test]
async fn management_server_rejects_and_closes_an_unexpected_right() {
    let config = Arc::new(test_config());
    let pool = Arc::new(BrokerConnectionPool::new(
        HOST_ID,
        config.revision(),
        config.runtime().clone(),
        FakeFactory::empty(Vec::new()),
    ));
    let (mut client, server) = UnixStream::pair().unwrap();
    let task = tokio::spawn(manage(
        server.into(),
        pool,
        config,
        Arc::new(tokio::sync::Notify::new()),
        Arc::new(tokio::sync::Notify::new()),
        Arc::new(BrokerAdmission::new(true)),
    ));
    let transferred = pipe();
    send_right(
        client.as_raw_fd(),
        transferred.writer.as_raw_fd(),
        ANCILLARY_MARKER,
    );
    drop(transferred.writer);
    let error = settle(task).await.unwrap().unwrap_err();
    assert_anyhow_ancillary(&error);
    assert_socket_eof(&mut client).await;
    assert_reader_eof(&transferred.reader);
}

#[tokio::test]
async fn management_client_rejects_and_closes_an_unexpected_right() {
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join(MANAGEMENT_SOCKET_FILE);
    let listener = UnixListener::bind(&path).unwrap();
    fs::set_permissions(&path, fs::Permissions::from_mode(PRIVATE_FILE_MODE)).unwrap();
    let transferred = pipe();
    let writer = transferred.writer;
    let (release_sender, release_receiver) = tokio::sync::oneshot::channel();
    let server = tokio::spawn(async move {
        let (mut stream, _) = listener.accept().await.unwrap();
        let _: super::super::BrokerRequestEnvelope =
            crate::protocol::read_frame(&mut stream).await.unwrap();
        send_right(stream.as_raw_fd(), writer.as_raw_fd(), ANCILLARY_MARKER);
        release_receiver.await.unwrap();
    });

    let error = settle(BrokerClient::new(path).status()).await.unwrap_err();
    let BrokerClientError::Protocol(ProtocolError::Io(transport)) = &error else {
        panic!("{ANCILLARY_ERROR_EXPECTED}: {error}")
    };
    assert_io_ancillary(transport);
    release_sender.send(()).unwrap();
    settle(server).await.unwrap();
    assert_reader_eof(&transferred.reader);
}

async fn assert_socket_eof(stream: &mut UnixStream) {
    let mut byte = u8::default();
    assert_eq!(
        settle(stream.read(std::slice::from_mut(&mut byte)))
            .await
            .unwrap(),
        usize::default()
    );
}

fn assert_anyhow_ancillary(error: &anyhow::Error) {
    let transport = error
        .chain()
        .find_map(|cause| cause.downcast_ref::<std::io::Error>())
        .unwrap();
    assert_io_ancillary(transport);
}

fn assert_io_ancillary(error: &std::io::Error) {
    assert!(
        error
            .get_ref()
            .is_some_and(|cause| cause.is::<UnexpectedAncillaryData>())
    );
}

async fn settle<F: Future>(future: F) -> F::Output {
    tokio::time::timeout(TRANSPORT_SETTLEMENT_TIMEOUT, future)
        .await
        .expect(TRANSPORT_SETTLEMENT_TIMEOUT_MESSAGE)
}

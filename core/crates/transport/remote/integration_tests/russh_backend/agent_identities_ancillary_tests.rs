use super::super::*;
use crate::local_unix::UnexpectedAncillaryData;
use crate::local_unix::test_support::{assert_reader_eof, pipe, send_right};
use crate::remote::RemoteError;
use std::future::Future;
use std::os::fd::AsRawFd;
use std::time::Duration;
use tokio::io::AsyncReadExt;

const ANCILLARY_AGENT_SOCKET: &str = "ancillary-agent.sock";
const ANCILLARY_MARKER: u8 = b'A';
const TRANSPORT_SETTLEMENT_TIMEOUT: Duration = Duration::from_secs(2);
const TRANSPORT_SETTLEMENT_TIMEOUT_MESSAGE: &str = "SSH agent transport test did not settle";
const ANCILLARY_ERROR_EXPECTED: &str = "SSH agent did not report forbidden ancillary data";

#[tokio::test]
async fn agent_client_rejects_and_closes_an_unexpected_right() {
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join(ANCILLARY_AGENT_SOCKET);
    let listener = tokio::net::UnixListener::bind(&path).unwrap();
    let transferred = pipe();
    let writer = transferred.writer;
    let (release_sender, release_receiver) = tokio::sync::oneshot::channel();
    let server = tokio::spawn(async move {
        let (mut stream, _) = listener.accept().await.unwrap();
        let request_bytes = SSH_AGENT_LENGTH_PREFIX_BYTES
            + usize::try_from(SSH_AGENT_SINGLE_MESSAGE_BYTES).unwrap();
        let mut request = vec![u8::default(); request_bytes];
        stream.read_exact(&mut request).await.unwrap();
        send_right(stream.as_raw_fd(), writer.as_raw_fd(), ANCILLARY_MARKER);
        release_receiver.await.unwrap();
    });

    let agent = settle(connect_uds(path)).await.unwrap();
    let error = settle(request_bounded(agent)).await.err().unwrap();
    let RemoteError::Transport(detail) = error else {
        panic!("{ANCILLARY_ERROR_EXPECTED}")
    };
    assert_eq!(detail, UnexpectedAncillaryData.to_string());
    release_sender.send(()).unwrap();
    settle(server).await.unwrap();
    assert_reader_eof(&transferred.reader);
}

async fn settle<F: Future>(future: F) -> F::Output {
    tokio::time::timeout(TRANSPORT_SETTLEMENT_TIMEOUT, future)
        .await
        .expect(TRANSPORT_SETTLEMENT_TIMEOUT_MESSAGE)
}

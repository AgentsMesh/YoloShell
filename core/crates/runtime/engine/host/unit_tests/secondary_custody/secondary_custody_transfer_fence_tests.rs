use super::*;
use crate::protocol::RequestEnvelope;
use crate::pty_custody::{CustodyRequest, CustodyUnixStream};
#[cfg(target_os = "macos")]
use std::os::fd::{AsFd, FromRawFd, OwnedFd};

const OPERATION_COMPLETED_BEFORE_OPENING_HOLD_DETAIL: &str =
    "Host custody operation completed before the Worker held the accepted opening";
#[cfg(target_os = "macos")]
const PIPE_ENDPOINT_COUNT: usize = 2;

struct TransferFixture {
    host: Host,
    record: RuntimeRecord,
    listener: Option<UnixListener>,
    _directory: HostTestDirectory,
}

impl TransferFixture {
    fn new() -> Self {
        let directory = tempdir().unwrap();
        let host = Host::open(super::super::super::HostConfig::new(directory.path())).unwrap();
        let socket = directory.path().join(TEST_SOCKET_NAME);
        let listener = UnixListener::bind(&socket).unwrap();
        let record = runtime(&host, socket);
        persist_pending(&host, &record);
        Self {
            _directory: HostTestDirectory::capture(directory, &host.paths),
            host,
            record,
            listener: Some(listener),
        }
    }

    fn take_listener(&mut self) -> UnixListener {
        self.listener.take().unwrap()
    }
}

#[tokio::test]
async fn cancellation_after_opening_acceptance_fail_stops_host() {
    let mut fixture = TransferFixture::new();
    let listener = fixture.take_listener();
    let (opening_accepted, accepted) = tokio::sync::oneshot::channel();
    let (release, released) = tokio::sync::oneshot::channel();
    let server = tokio::spawn(async move {
        let (stream, _) = accept_opening(listener).await;
        opening_accepted.send(()).unwrap();
        released.await.unwrap();
        drop(stream);
    });
    let mut operation = Box::pin(fixture.host.ensure_secondary_custody(&fixture.record));

    tokio::select! {
        accepted = accepted => accepted.unwrap(),
        _ = &mut operation => panic!("{OPERATION_COMPLETED_BEFORE_OPENING_HOLD_DETAIL}"),
    }
    drop(operation);

    assert_fail_stopped(&fixture.host);
    release.send(()).unwrap();
    server.await.unwrap();
}

#[tokio::test]
async fn eof_while_receiving_rights_fail_stops_host() {
    let mut fixture = TransferFixture::new();
    let listener = fixture.take_listener();
    let server = tokio::spawn(async move {
        let (mut stream, request) = accept_opening(listener).await;
        write_offer(&mut stream, &request).await;
    });

    let error = fixture
        .host
        .ensure_secondary_custody(&fixture.record)
        .await
        .unwrap_err();

    assert!(matches!(error, WorkerRpcError::Transport(_)));
    assert_fail_stopped(&fixture.host);
    server.await.unwrap();
}

#[cfg(target_os = "macos")]
#[tokio::test]
async fn descriptor_import_rejection_fail_stops_host() {
    let mut fixture = TransferFixture::new();
    let listener = fixture.take_listener();
    let server = tokio::spawn(async move {
        let (mut stream, request) = accept_opening(listener).await;
        write_offer(&mut stream, &request).await;
        let descriptors = pipe();
        stream
            .send_rights(&[descriptors[0].as_fd(), descriptors[1].as_fd()])
            .await
            .unwrap();
        let _ = stream.read::<crate::pty_custody::CustodyHostAck>().await;
    });

    let error = fixture
        .host
        .ensure_secondary_custody(&fixture.record)
        .await
        .unwrap_err();

    assert!(matches!(error, WorkerRpcError::IdentityMismatch(_)));
    assert_fail_stopped(&fixture.host);
    server.await.unwrap();
}

async fn accept_opening(listener: UnixListener) -> (CustodyUnixStream, CustodyRequest) {
    let (accepted, _) = listener.accept().await.unwrap();
    let mut stream = CustodyUnixStream::from_accepted(accepted).unwrap();
    let opening: RequestEnvelope = stream.read().await.unwrap();
    let request = CustodyUnixStream::decode_opening(&opening).unwrap();
    (stream, request)
}

async fn write_offer(stream: &mut CustodyUnixStream, request: &CustodyRequest) {
    let digest = crate::sha256_digest::Sha256Digest::repeated(TEST_DIGEST_BYTE);
    let offer = crate::host::test_custody_offer(request, super::platform_descriptor_set(), digest);
    stream.write(&offer).await.unwrap();
}

fn assert_fail_stopped(host: &Host) {
    assert!(host.request_drain.is_draining());
    assert!(host.request_drain.fail_stop_detail().is_some());
}

#[cfg(target_os = "macos")]
fn pipe() -> [OwnedFd; PIPE_ENDPOINT_COUNT] {
    let mut descriptors = [libc::c_int::default(); PIPE_ENDPOINT_COUNT];
    assert_eq!(
        unsafe { libc::pipe(descriptors.as_mut_ptr()) },
        libc::c_int::default()
    );
    descriptors.map(|descriptor| unsafe { OwnedFd::from_raw_fd(descriptor) })
}

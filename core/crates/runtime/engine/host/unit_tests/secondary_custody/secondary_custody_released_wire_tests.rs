use super::*;
use crate::paths::atomic_write_json;
use crate::pty_custody::{
    CustodyDescriptorSet, CustodyHostAck, CustodyLeaseId, CustodyOffer, CustodyRequest,
    CustodyRuntimeIdentity,
};
use crate::sha256_digest::Sha256Digest;
use std::os::fd::{AsFd, AsRawFd, FromRawFd, OwnedFd};

const CONTROLLER_GENERATION_INCREMENT: u64 = 1;
const PIPE_ENDPOINT_COUNT: usize = 2;
const STALE_START_TIME_INCREMENT: u64 = 1;
const SYSTEM_CALL_FAILURE: libc::c_int = -1;

#[tokio::test(flavor = "current_thread")]
async fn released_successor_wire_right_is_rejected_and_closed() {
    let directory = tempfile::tempdir().unwrap();
    advance_controller_generation(directory.path());
    let host = Host::open(super::super::super::HostConfig::new(directory.path())).unwrap();
    let socket = directory.path().join(TEST_SOCKET_NAME);
    let listener = UnixListener::bind(&socket).unwrap();
    let record = runtime(&host, socket);
    let digest = Sha256Digest::repeated(TEST_DIGEST_BYTE);
    let _gone = persist_gone_predecessor(&host, &record, digest);
    let [reader, writer] = pipe();
    let server = tokio::spawn(async move {
        let (stream, _) = listener.accept().await.unwrap();
        let mut stream = CustodyUnixStream::from_accepted(stream).unwrap();
        let opening: crate::protocol::RequestEnvelope = stream.read().await.unwrap();
        let request = CustodyUnixStream::decode_opening(&opening).unwrap();
        let offer = crate::host::test_custody_offer(&request, CustodyDescriptorSet::ReleasedSuccessor, digest);
        stream.write(&offer).await.unwrap();
        stream.send_rights(&[writer.as_fd()]).await.unwrap();
        drop(writer);
        let _ = stream.read::<CustodyHostAck>().await;
    });

    let error = host.ensure_secondary_custody(&record).await.unwrap_err();
    assert!(matches!(error, WorkerRpcError::Transport(_)));
    server.await.unwrap();
    assert_writer_copies_closed(reader);
}

fn advance_controller_generation(home: &std::path::Path) {
    let host = Host::open(super::super::super::HostConfig::new(home)).unwrap();
    drop(host);
}

fn persist_gone_predecessor(
    host: &Host,
    record: &RuntimeRecord,
    digest: Sha256Digest,
) -> crate::process::ProcessIdentityRelationGuard {
    let pending = persist_pending(host, record);
    let current = CustodyHolderIdentity::capture().unwrap();
    let predecessor = CustodyHolderIdentity {
        pid: current.pid,
        start_time: current
            .start_time
            .saturating_add(STALE_START_TIME_INCREMENT),
    };
    let gone = crate::process::install_test_process_identity_relation(
        predecessor.pid,
        predecessor.start_time,
        crate::process::ProcessIdentityRelation::Gone,
    );
    let generation = host
        .controller_generation
        .checked_sub(CONTROLLER_GENERATION_INCREMENT)
        .unwrap();
    let request = CustodyRequest::new(
        CustodyLeaseId::new(),
        CustodyRuntimeIdentity::from_record(record),
        predecessor,
        record.auth_token.clone(),
        generation,
    );
    let offer = crate::host::test_custody_offer(&request, super::platform_descriptor_set(), digest);
    let armed = crate::host::test_arm_custody(&pending, &request, &offer).unwrap();
    atomic_write_json(&host.paths.pty_custody(record.terminal_id), &armed).unwrap();
    gone
}

fn pipe() -> [OwnedFd; PIPE_ENDPOINT_COUNT] {
    let mut descriptors = [libc::c_int::default(); PIPE_ENDPOINT_COUNT];
    assert_eq!(
        unsafe { libc::pipe(descriptors.as_mut_ptr()) },
        libc::c_int::default()
    );
    descriptors.map(|descriptor| unsafe { OwnedFd::from_raw_fd(descriptor) })
}

fn assert_writer_copies_closed(reader: OwnedFd) {
    let descriptor = reader.as_raw_fd();
    let flags = unsafe { libc::fcntl(descriptor, libc::F_GETFL) };
    assert_ne!(flags, SYSTEM_CALL_FAILURE);
    assert_ne!(
        unsafe { libc::fcntl(descriptor, libc::F_SETFL, flags | libc::O_NONBLOCK) },
        SYSTEM_CALL_FAILURE
    );
    let mut byte = u8::default();
    assert_eq!(
        unsafe {
            libc::read(
                descriptor,
                std::ptr::addr_of_mut!(byte).cast(),
                std::mem::size_of_val(&byte),
            )
        },
        libc::ssize_t::default()
    );
}

use super::*;
use crate::paths::constants::MAX_DURABLE_JSON_BYTES;
use crate::protocol::RequestEnvelope;
use crate::pty_custody::{
    CustodyDescriptorSet, CustodyGuardianBinding, CustodyLeaseId, CustodyOffer, CustodyRequest,
    CustodyRuntimeIdentity,
};
use crate::sha256_digest::Sha256Digest;
use std::future::pending;
use std::os::unix::fs::PermissionsExt;
use std::time::Duration;
use tokio::sync::oneshot;
use tokio::time::{Instant, advance};
use uuid::Uuid;

const TEST_STALL_TIMEOUT: Duration = Duration::from_secs(1);
const TEST_WORKER_PID: u32 = 41_101;
const TEST_WORKER_START_TIME: u64 = 51_101;
const TEST_CHILD_PID: u32 = 41_102;
const TEST_CHILD_START_TIME: u64 = 51_102;
const TEST_BOOT_ID: &str = "recovery-client-deadline-boot";
const TEST_AUTH_TOKEN: &str = "recovery-client-deadline-token";
const TEST_CONTROLLER_GENERATION: u64 = 1;
const TEST_READER_HANDLE: u64 = 1;
const TEST_WRITER_HANDLE: u64 = 2;
const TEST_DIGEST_BYTE: u8 = 0x51;
const TEST_STATE_FILE: &str = "runtime.json";
const TEST_STATE_BYTES: &[u8] = b"{}";
const TEST_PRIVATE_FILE_MODE: u32 = 0o600;
const PREMATURE_RECOVERY_DETAIL: &str = "recovery finished before its opening stalled";
const UNEXPECTED_STALL_ERROR_DETAIL: &str = "unexpected recovery stall error";

#[tokio::test(start_paused = true)]
async fn permanent_recovery_stall_exhausts_one_absolute_deadline() {
    let (_directory, request) = request_fixture();
    let (left, right) = tokio::net::UnixStream::pair().unwrap();
    let mut client = CustodyUnixStream::from_accepted(left).unwrap();
    let mut server = CustodyUnixStream::from_accepted(right).unwrap();
    let (opening_seen, mut opening_received) = oneshot::channel();
    let server_task = tokio::spawn(async move {
        let _: RequestEnvelope = server.read().await.unwrap();
        opening_seen.send(()).unwrap();
        pending::<()>().await;
    });

    let deadline = Instant::now() + TEST_STALL_TIMEOUT;
    let mut exchange = Box::pin(exchange(&mut client, &request));
    tokio::select! {
        result = &mut exchange => panic!("{PREMATURE_RECOVERY_DETAIL}: {result:?}"),
        result = &mut opening_received => result.unwrap(),
    }
    assert!(matches!(
        futures::poll!(&mut exchange),
        std::task::Poll::Pending
    ));
    let operation = within_deadline(deadline, exchange);
    tokio::pin!(operation);
    assert!(matches!(
        futures::poll!(&mut operation),
        std::task::Poll::Pending
    ));

    advance(TEST_STALL_TIMEOUT).await;
    let error = operation.await.unwrap_err();
    assert!(
        format!("{error:#}").contains(RECOVERY_TIMEOUT_DETAIL),
        "{UNEXPECTED_STALL_ERROR_DETAIL}: {error:#}"
    );

    server_task.abort();
    let _ = server_task.await;
}

fn request_fixture() -> (tempfile::TempDir, GuardianRecoveryRequest) {
    let directory = tempfile::tempdir().unwrap();
    let state_path = directory.path().join(TEST_STATE_FILE);
    std::fs::write(&state_path, TEST_STATE_BYTES).unwrap();
    std::fs::set_permissions(
        &state_path,
        std::fs::Permissions::from_mode(TEST_PRIVATE_FILE_MODE),
    )
    .unwrap();
    let metadata = std::fs::metadata(&state_path).unwrap();
    let identity =
        FileIdentity::inspect_bounded(&metadata, &state_path, MAX_DURABLE_JSON_BYTES).unwrap();
    let runtime = CustodyRuntimeIdentity {
        terminal_id: Uuid::new_v4(),
        runtime_epoch: Uuid::new_v4(),
        worker_id: Uuid::new_v4(),
        worker_pid: TEST_WORKER_PID,
        worker_start_time: TEST_WORKER_START_TIME,
        child_pid: TEST_CHILD_PID,
        child_start_time: TEST_CHILD_START_TIME,
        boot_id: TEST_BOOT_ID.to_owned(),
    };
    let holder = CustodyHolderIdentity::capture().unwrap();
    let request = CustodyRequest::new(
        CustodyLeaseId::new(),
        runtime.clone(),
        holder,
        TEST_AUTH_TOKEN.to_owned(),
        TEST_CONTROLLER_GENERATION,
    );
    let offer = CustodyOffer::new_with_profile(
        &request,
        CustodyDescriptorSet::macos_process_capability(TEST_READER_HANDLE, TEST_WRITER_HANDLE),
        crate::pty_custody::CustodyProfile::ActiveRunning,
        Sha256Digest::repeated(TEST_DIGEST_BYTE),
    )
    .unwrap();
    let guardian = CustodyGuardianBinding::new(Uuid::new_v4(), holder, &request, &offer).unwrap();
    let durable = CustodyDurableRecord::pending(runtime)
        .prepare_guardian(&request, &offer, guardian)
        .unwrap()
        .arm_prepared_guardian(&request, &offer, guardian)
        .unwrap();
    (
        directory,
        GuardianRecoveryRequest::new(holder, guardian, identity, durable, identity),
    )
}

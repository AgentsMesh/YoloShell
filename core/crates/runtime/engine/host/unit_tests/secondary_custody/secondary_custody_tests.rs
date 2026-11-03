use super::*;
use crate::host::test_directory::HostTestDirectory;
use crate::model::{BootstrapDelivery, RuntimeRecord};
use crate::paths::atomic_write_json;
use crate::pty_custody::{
    CustodyDescriptorSet, CustodyDurableRecord, CustodyOffer, CustodyUnixStream,
};
use std::os::unix::fs::PermissionsExt;
use tempfile::{TempDir, tempdir as raw_tempdir};
use tokio::net::UnixListener;
#[cfg(target_os = "macos")]
#[path = "secondary_custody_activation_admission_tests.rs"]
mod activation_admission_tests;
#[cfg(target_os = "macos")]
#[path = "secondary_custody_activation_tests.rs"]
mod activation_tests;
#[cfg(target_os = "macos")]
#[path = "secondary_custody_active_retired_validation_tests.rs"]
mod active_retired_validation_tests;
#[path = "secondary_custody_attachment_tests.rs"]
mod attachment_tests;
#[cfg(target_os = "macos")]
#[path = "secondary_custody_control_tests.rs"]
mod control_tests;
#[path = "secondary_custody_error_containment_tests.rs"]
mod error_containment_tests;
#[path = "secondary_custody_fail_stop_containment_tests.rs"]
mod fail_stop_containment_tests;
#[path = "secondary_custody_fail_stop_tests.rs"]
mod fail_stop_tests;
#[cfg(target_os = "macos")]
#[path = "secondary_custody_failure_tests.rs"]
mod failure_tests;
#[cfg(target_os = "macos")]
#[path = "../../test_support/unit/secondary_custody/secondary_custody_handshake_test_support.rs"]
mod handshake_support;
#[cfg(target_os = "macos")]
#[path = "secondary_custody_observation_tests.rs"]
mod observation_tests;
#[path = "secondary_custody_recovery_admission_tests.rs"]
mod recovery_admission_tests;
#[cfg(target_os = "macos")]
#[path = "secondary_custody_recovery_tests.rs"]
mod recovery_tests;
#[path = "secondary_custody_released_validation_tests.rs"]
mod released_validation_tests;
#[path = "secondary_custody_released_wire_tests.rs"]
mod released_wire_tests;
#[path = "secondary_custody_retirement_receipt_tests.rs"]
mod retirement_receipt_tests;
#[cfg(target_os = "macos")]
#[path = "secondary_custody_rollback_eof_tests.rs"]
mod rollback_eof_tests;
#[cfg(target_os = "macos")]
#[path = "secondary_custody_rollback_tests.rs"]
mod rollback_tests;
#[cfg(target_os = "macos")]
#[path = "secondary_custody_status_timeout_tests.rs"]
mod status_timeout_tests;
#[cfg(target_os = "macos")]
#[path = "../../test_support/unit/secondary_custody/secondary_custody_test_support.rs"]
mod support;
include!("secondary_custody_transfer_fence_test_wiring.rs");
include!("../../test_support/unit/secondary_custody/secondary_custody_test_fixture.rs");

fn tempdir() -> std::io::Result<TempDir> {
    let directory = raw_tempdir()?;
    std::fs::set_permissions(
        directory.path(),
        std::fs::Permissions::from_mode(crate::host::TEST_PRIVATE_DIRECTORY_MODE),
    )?;
    Ok(directory)
}

#[tokio::test]
async fn missing_durable_custody_fails_closed_before_connecting() {
    let directory = tempdir().unwrap();
    let host = Host::open(super::super::HostConfig::new(directory.path())).unwrap();
    let record = runtime(&host, directory.path().join(TEST_SOCKET_NAME));
    host.paths.ensure_session(record.terminal_id).unwrap();

    let error = host.ensure_secondary_custody(&record).await.unwrap_err();

    assert!(matches!(error, WorkerRpcError::Transport(_)));
    assert!(!host.secondary_custody.ready_for(
        &record,
        host.secondary_custody_holder,
        host.controller_generation,
    ));
}

#[tokio::test]
async fn socket_peer_must_be_the_exact_persisted_worker() {
    let directory = tempdir().unwrap();
    let host = Host::open(super::super::HostConfig::new(directory.path())).unwrap();
    let socket = directory.path().join(TEST_SOCKET_NAME);
    let listener = UnixListener::bind(&socket).unwrap();
    let mut record = runtime(&host, socket);
    record.worker_start_time = record.worker_start_time.saturating_add(1);
    persist_pending(&host, &record);
    let server = tokio::spawn(async move {
        let _accepted = listener.accept().await.unwrap();
    });

    let error = host.ensure_secondary_custody(&record).await.unwrap_err();

    assert!(matches!(error, WorkerRpcError::IdentityMismatch(_)));
    server.await.unwrap();
}

#[tokio::test]
async fn offer_with_another_platform_descriptor_set_is_rejected_before_rights() {
    let directory = tempdir().unwrap();
    let host = Host::open(super::super::HostConfig::new(directory.path())).unwrap();
    let socket = directory.path().join(TEST_SOCKET_NAME);
    let listener = UnixListener::bind(&socket).unwrap();
    let record = runtime(&host, socket);
    persist_pending(&host, &record);
    let server = tokio::spawn(async move {
        let (stream, _) = listener.accept().await.unwrap();
        let mut stream = CustodyUnixStream::from_accepted(stream).unwrap();
        let opening: crate::protocol::RequestEnvelope = stream.read().await.unwrap();
        let request = CustodyUnixStream::decode_opening(&opening).unwrap();
        let descriptor_set = mismatched_descriptor_set();
        let digest = crate::sha256_digest::Sha256Digest::repeated(TEST_DIGEST_BYTE);
        stream
            .write(&crate::host::test_custody_offer(
                &request,
                descriptor_set,
                digest,
            ))
            .await
            .unwrap();
    });

    let error = host.ensure_secondary_custody(&record).await.unwrap_err();

    assert!(matches!(error, WorkerRpcError::IdentityMismatch(_)));
    server.await.unwrap();
}

#[cfg(target_os = "macos")]
#[tokio::test]
async fn current_worker_eof_does_not_use_child_absence_as_settlement_authority() {
    let mut fixture = support::MacosHandshakeFixture::new(None);
    let record = fixture.record.clone();
    let server = fixture.serve();

    let worker = fixture.host.custodied_worker_stream(&record).await.unwrap();
    fixture.settle_runtime_child();
    drop(worker);
    server.await.unwrap();

    let lease = fixture
        .host
        .secondary_custody
        .current(&CustodyRuntimeIdentity::from_record(&record))
        .unwrap();
    tokio::time::timeout(fixture.host.worker_ready_timeout, async {
        while fixture.host.request_drain.fail_stop_detail().is_none() {
            tokio::time::sleep(fixture.host.worker_ready_poll_interval).await;
        }
    })
    .await
    .unwrap();
    assert!(!fixture.host.secondary_custody.ready_for(
        &record,
        fixture.host.secondary_custody_holder,
        fixture.host.controller_generation,
    ));
    assert_eq!(lease.state(), SecondaryCustodyState::Disconnected);
    assert!(lease.retains_platform());
    assert!(!lease.settlement_is_active());
}

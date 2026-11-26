use crate::host::tests::*;
use super::*;

const RESUME_EXITED_SOCKET_NAME: &str = "resume-exited.sock";
const RESUME_UNSETTLED_SOCKET_NAME: &str = "resume-unsettled.sock";
const RETIRED_WORKER_SETTLEMENT_TEST_TIMEOUT: Duration =
    crate::session_timing::WORKER_SHUTDOWN_SETTLEMENT_TIMEOUT
        .saturating_add(HOST_BLOCKING_TEST_TIMEOUT);

#[tokio::test]
async fn explicit_resume_does_not_rpc_a_terminally_retired_custody_lease() {
    for state in resume_retired_custody_states() {
        let (_directory, host, mut manifest, record) = retired_resume_fixture(state);
        let result = host.retire_previous_runtime(&mut manifest).await;
        assert!(
            result.is_ok(),
            "terminally retired custody must settle from durable evidence: {result:?}"
        );

        let error = host
            .resume_session(exact_recreate_request(
                &manifest,
                &record,
                TerminalRuntimeStatus::Exited,
            ))
            .await
            .unwrap_err();
        assert_ne!(
            error.code,
            crate::protocol::error_code::PROTOCOL_INCOMPATIBLE
        );
    }
}

#[tokio::test]
async fn terminally_retired_custody_stays_fail_closed_while_worker_is_live() {
    for state in resume_retired_custody_states() {
        let directory = tempdir().unwrap();
        let host = unspawnable_resume_host(&directory);
        let manifest = fixture_manifest(directory.path());
        let mut record = dead_resume_runtime(&host, &manifest);
        record.worker_pid = std::process::id();
        record.worker_start_time = process::process_start_time(record.worker_pid).unwrap();
        persist_host_session(&host, &manifest, &record);
        persist_archive_exit(&host, &record, record.runtime_epoch);
        host.secondary_custody
            .install_test_retired_lease(&host, &record, state);

        let mut manifest_for_resume = manifest.clone();
        let result = tokio::time::timeout(
            RETIRED_WORKER_SETTLEMENT_TEST_TIMEOUT,
            host.retire_previous_runtime(&mut manifest_for_resume),
        )
        .await
        .expect("live-worker retirement remained bounded");
        let error = result.unwrap_err().into_explicit_error();

        assert_eq!(error.code, crate::protocol::error_code::WORKER_UNREACHABLE);
        assert!(error.retryable);
        assert_eq!(
            lock_recover(&host.catalog)
                .get(manifest.terminal_id)
                .unwrap()
                .unwrap()
                .recovery_state,
            RecoveryState::Uncertain
        );
    }
}

#[tokio::test]
async fn explicit_resume_settles_a_worker_exiting_after_exited_status() {
    let directory = tempdir().unwrap();
    let host = unspawnable_resume_host(&directory);
    let manifest = fixture_manifest(directory.path());
    let socket = directory.path().join(RESUME_EXITED_SOCKET_NAME);
    let mut worker = ArchiveWorkerProcessFixture::spawn(
        socket.clone(),
        ArchiveWorkerFixtureMode::ExitedThenExitBoth,
    );
    let mut record = host_runtime_record(&host, manifest.terminal_id, socket);
    worker.configure(&mut record);
    persist_host_session(&host, &manifest, &record);
    #[cfg(target_os = "linux")]
    let _containment = crate::host::linux_retirement_evidence::capture_live(&host, &record);
    persist_archive_exit(&host, &record, record.runtime_epoch);

    let result = tokio::time::timeout(
        HOST_BLOCKING_TEST_TIMEOUT,
        host.resume_session(exact_recreate_request(
            &manifest,
            &record,
            TerminalRuntimeStatus::Exited,
        )),
    )
    .await
    .expect("explicit resume settlement remained bounded");
    let error = result.unwrap_err();

    assert_eq!(error.code, crate::protocol::error_code::WORKER_START_FAILED);
    assert!(!process::process_identity_matches(
        record.worker_pid,
        record.worker_start_time
    ));
    assert!(!process::process_identity_matches(
        record.child_pid,
        record.child_start_time
    ));
    worker.reap_completed_worker();
}

#[tokio::test]
async fn explicit_resume_rejects_a_disconnected_worker_with_a_live_child() {
    let directory = tempdir().unwrap();
    let host = unspawnable_resume_host(&directory);
    let manifest = fixture_manifest(directory.path());
    let socket = directory.path().join(RESUME_UNSETTLED_SOCKET_NAME);
    let mut worker = ArchiveWorkerProcessFixture::spawn(
        socket.clone(),
        ArchiveWorkerFixtureMode::ExitedThenExitWorker,
    );
    let mut record = host_runtime_record(&host, manifest.terminal_id, socket);
    worker.configure(&mut record);
    persist_host_session(&host, &manifest, &record);
    persist_archive_exit(&host, &record, record.runtime_epoch);

    let result = tokio::time::timeout(
        HOST_BLOCKING_TEST_TIMEOUT,
        host.resume_session(exact_recreate_request(
            &manifest,
            &record,
            TerminalRuntimeStatus::Exited,
        )),
    )
    .await
    .expect("unsettled explicit resume remained bounded");
    let error = result.unwrap_err();

    assert_eq!(error.code, crate::protocol::error_code::WORKER_UNREACHABLE);
    assert!(error.retryable);
    assert!(!host.paths.worker_launch(manifest.terminal_id).exists());
    assert_eq!(
        lock_recover(&host.catalog)
            .get(manifest.terminal_id)
            .unwrap()
            .unwrap()
            .recovery_state,
        RecoveryState::Uncertain
    );
    assert!(process::process_identity_matches(
        record.child_pid,
        record.child_start_time
    ));
    worker.reap_completed_worker();
    worker.terminate_child();
}

fn retired_resume_fixture(
    state: crate::host::secondary_custody::TestRetiredCustodyState,
) -> (tempfile::TempDir, Host, TerminalManifest, RuntimeRecord) {
    let directory = tempdir().unwrap();
    let host = unspawnable_resume_host(&directory);
    let manifest = fixture_manifest(directory.path());
    let record = dead_resume_runtime(&host, &manifest);
    persist_host_session(&host, &manifest, &record);
    persist_archive_exit(&host, &record, record.runtime_epoch);
    host.secondary_custody
        .install_test_retired_lease(&host, &record, state);
    (directory, host, manifest, record)
}

fn resume_retired_custody_states() -> [crate::host::secondary_custody::TestRetiredCustodyState; 2] {
    [
        crate::host::secondary_custody::TestRetiredCustodyState::Released,
        crate::host::secondary_custody::TestRetiredCustodyState::Superseded,
    ]
}

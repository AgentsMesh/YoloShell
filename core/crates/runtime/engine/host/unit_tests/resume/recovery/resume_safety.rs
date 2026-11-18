use crate::host::tests::*;
use super::*;

const RESUME_SAFETY_MISSING_PID: u32 = u32::MAX;
const RESUME_SAFETY_MISSING_PROCESS_START_TIME: u64 = 1;
const RESUME_SAFETY_PREVIOUS_BOOT_ID: &str = "resume-safety-previous-boot";
const RESUME_SAFETY_PREVIOUS_BOOT_SOCKET: &str = "previous-boot.sock";
const RESUME_SAFETY_REUSED_WORKER_SOCKET: &str = "reused-worker.sock";

#[tokio::test]
async fn explicit_resume_fences_a_live_child_before_watchdog_reconciliation() {
    let directory = tempdir().unwrap();
    let mut config = HostConfig::new(directory.path());
    config.runtime_executable = Some(directory.path().join("must-not-spawn"));
    let host = Host::open(config).unwrap();
    let manifest = fixture_manifest(directory.path());
    let mut record = host_runtime_record(
        &host,
        manifest.terminal_id,
        directory.path().join("dead-worker.sock"),
    );
    record.worker_pid = RESUME_SAFETY_MISSING_PID;
    record.worker_start_time = RESUME_SAFETY_MISSING_PROCESS_START_TIME;
    persist_host_session(&host, &manifest, &record);

    let error = host
        .resume_session(exact_recreate_request(
            &manifest,
            &record,
            TerminalRuntimeStatus::Unreachable,
        ))
        .await
        .unwrap_err();

    assert_eq!(error.code, crate::protocol::error_code::WORKER_UNREACHABLE);
    assert!(error.retryable);
    assert!(!host.paths.worker_launch(manifest.terminal_id).exists());
    let stored = lock_recover(&host.catalog)
        .get(manifest.terminal_id)
        .unwrap()
        .unwrap();
    assert_eq!(stored.recovery_state, RecoveryState::Uncertain);
}

#[tokio::test]
async fn explicit_resume_never_contacts_a_reused_worker_pid() {
    let directory = tempdir().unwrap();
    let host = unspawnable_resume_host(&directory);
    let manifest = fixture_manifest(directory.path());
    let current_pid = std::process::id();
    let current_start_time = process::process_start_time(current_pid).unwrap();
    assert_ne!(current_start_time, RESUME_SAFETY_MISSING_PROCESS_START_TIME);
    let mut record = host_runtime_record(
        &host,
        manifest.terminal_id,
        directory.path().join(RESUME_SAFETY_REUSED_WORKER_SOCKET),
    );
    record.worker_pid = current_pid;
    record.worker_start_time = RESUME_SAFETY_MISSING_PROCESS_START_TIME;
    record.child_pid = current_pid;
    record.child_start_time = current_start_time;
    persist_host_session(&host, &manifest, &record);

    let error = host
        .resume_session_locked(manifest.terminal_id, ResumeSelection::explicit(true))
        .await
        .unwrap_err()
        .into_explicit_error();

    assert_eq!(error.code, crate::protocol::error_code::WORKER_UNREACHABLE);
    assert!(error.retryable);
    assert!(process::process_identity_matches(
        current_pid,
        current_start_time
    ));
    assert!(!host.paths.worker_launch(manifest.terminal_id).exists());
}

#[tokio::test]
async fn explicit_resume_requires_durable_previous_runtime_exit() {
    let directory = tempdir().unwrap();
    let host = unspawnable_resume_host(&directory);
    let manifest = fixture_manifest(directory.path());
    let record = dead_resume_runtime(&host, &manifest);
    persist_host_session(&host, &manifest, &record);

    let error = host
        .resume_session(exact_recreate_request(
            &manifest,
            &record,
            TerminalRuntimeStatus::Lost,
        ))
        .await
        .unwrap_err();

    assert_eq!(error.code, crate::protocol::error_code::WORKER_UNREACHABLE);
    assert!(error.retryable);
    assert!(!host.paths.worker_launch(manifest.terminal_id).exists());
    let stored = lock_recover(&host.catalog)
        .get(manifest.terminal_id)
        .unwrap()
        .unwrap();
    assert_eq!(stored.recovery_state, RecoveryState::Uncertain);
}

#[tokio::test]
async fn explicit_resume_skips_process_observation_for_a_previous_boot() {
    let directory = tempdir().unwrap();
    let host = unspawnable_resume_host(&directory);
    let manifest = fixture_manifest(directory.path());
    let current_pid = std::process::id();
    let current_start_time = process::process_start_time(current_pid).unwrap();
    let mut record = host_runtime_record(
        &host,
        manifest.terminal_id,
        directory.path().join(RESUME_SAFETY_PREVIOUS_BOOT_SOCKET),
    );
    record.boot_id = RESUME_SAFETY_PREVIOUS_BOOT_ID.into();
    record.worker_pid = current_pid;
    record.worker_start_time = current_start_time;
    record.child_pid = current_pid;
    record.child_start_time = current_start_time;
    persist_host_session(&host, &manifest, &record);

    let error = host
        .resume_session_locked(manifest.terminal_id, ResumeSelection::explicit(true))
        .await
        .unwrap_err()
        .into_explicit_error();

    assert_eq!(error.code, crate::protocol::error_code::WORKER_START_FAILED);
    assert!(process::process_identity_matches(
        current_pid,
        current_start_time
    ));
}

#[tokio::test]
async fn unknown_boot_identity_never_proves_that_a_live_runtime_is_gone() {
    let directory = tempdir().unwrap();
    let mut host = Host::open(HostConfig::new(directory.path())).unwrap();
    host.boot_id = format!("{}host", crate::process::unknown_boot_id_for_test());
    let manifest = fixture_manifest(directory.path());
    let mut record = host_runtime_record(
        &host,
        manifest.terminal_id,
        directory.path().join("unknown-boot.sock"),
    );
    record.boot_id = format!("{}worker", crate::process::unknown_boot_id_for_test());
    persist_host_session(&host, &manifest, &record);

    assert_eq!(
        host.automatic_restore_decision(&manifest),
        AutomaticRestore::BlockedByRuntime
    );
    assert!(host.live_runtime_record(manifest.terminal_id).await.is_ok());
}

#[test]
fn same_boot_live_worker_blocks_restore_after_child_exit() {
    let directory = tempdir().unwrap();
    let host = Host::open(HostConfig::new(directory.path())).unwrap();
    let manifest = fixture_manifest(directory.path());
    let mut record = host_runtime_record(
        &host,
        manifest.terminal_id,
        directory.path().join("live-worker.sock"),
    );
    record.child_pid = RESUME_SAFETY_MISSING_PID;
    record.child_start_time = RESUME_SAFETY_MISSING_PROCESS_START_TIME;
    persist_host_session(&host, &manifest, &record);

    assert!(!process_identity_is_gone(
        record.worker_pid,
        record.worker_start_time
    ));
    assert!(process_identity_is_gone(
        record.child_pid,
        record.child_start_time
    ));
    assert_eq!(
        host.automatic_restore_decision(&manifest),
        AutomaticRestore::BlockedByRuntime
    );
}

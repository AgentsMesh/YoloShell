use super::*;
use crate::host::tests::*;

const ACTIVE_CREATION_PREVIOUS_BOOT: &str = "active-creation-previous-boot";

pub(super) fn interleaving_request(directory: &Path, token: Uuid) -> CreateTerminal {
    CreateTerminal {
        creation_token: token,
        name: Some("interleaving".into()),
        cwd: directory.to_path_buf(),
        cols: Some(88),
        rows: Some(27),
        initial_input: None,
        context: None,
    }
}

pub(super) fn reserve_interleaving(host: &Host, request: &CreateTerminal) -> CreationBinding {
    let fingerprint = CreationSpec::fingerprint(request).unwrap();
    let spec = CreationSpec::resolve(request, fingerprint).unwrap();
    host.reserve_creation(
        &host.paths.home().join(CREATION_LEDGER_FILE_NAME),
        request.creation_token,
        spec,
    )
    .unwrap()
}

#[test]
fn duplicate_create_reread_observes_finalize_receipt_without_dual_identity() {
    let directory = tempdir().unwrap();
    let host = Host::open(HostConfig::new(directory.path())).unwrap();
    let token = Uuid::new_v4();
    let request = interleaving_request(directory.path(), token);
    let mut binding = reserve_interleaving(&host, &request);
    let path = host.paths.home().join(CREATION_LEDGER_FILE_NAME);
    let launch = lock_recover(&host.creation_capabilities)
        .prepare_launch(&path, binding.result)
        .unwrap();
    lock_recover(&host.creation_capabilities)
        .set_active_phase(&path, binding.result, CreationPhase::Ready)
        .unwrap();
    binding.phase = CreationPhase::Ready;
    binding.launch = Some(launch);
    let first_read = host
        .existing_creation(&path, token, &binding.spec.request_fingerprint)
        .unwrap()
        .unwrap();
    assert_eq!(first_read.result, binding.result);
    lock_recover(&host.creation_capabilities)
        .finalize(
            &path,
            FinalizeCreationRequest {
                creation_token: token,
                terminal_id: binding.result.terminal_id,
                creation_capability: binding.result.creation_capability,
            },
            &host.creation_receipts,
        )
        .unwrap();
    let error = host
        .existing_creation(&path, token, &binding.spec.request_fingerprint)
        .unwrap_err();
    assert_eq!(error.code, crate::protocol::error_code::CONFLICT);
    assert!(
        lock_recover(&host.creation_capabilities)
            .bindings
            .is_empty()
    );
    assert_eq!(
        host.creation_receipts
            .lookup_binding(&binding)
            .unwrap()
            .unwrap()
            .completion,
        CreationCompletion::Finalized
    );
}

#[test]
fn same_token_different_fingerprint_conflicts_before_another_reservation() {
    let directory = tempdir().unwrap();
    let host = Host::open(HostConfig::new(directory.path())).unwrap();
    let token = Uuid::new_v4();
    let request = interleaving_request(directory.path(), token);
    reserve_interleaving(&host, &request);
    let error = host
        .existing_creation(
            &host.paths.home().join(CREATION_LEDGER_FILE_NAME),
            token,
            &yolo_crypto::sha256_hex(b"different request"),
        )
        .unwrap_err();
    assert_eq!(error.code, crate::protocol::error_code::CONFLICT);
    assert_eq!(lock_recover(&host.creation_capabilities).bindings.len(), 1);
}

#[tokio::test]
async fn different_session_operation_locks_do_not_block_each_other() {
    let locks = SessionLocks::default();
    let first = Uuid::new_v4();
    let second = Uuid::new_v4();
    let _first_guard = locks.lock(first).await;
    let second_guard = tokio::time::timeout(WORKER_READY_POLL_INTERVAL, locks.lock(second))
        .await
        .expect("an independent session lock must be immediately available");
    drop(second_guard);
    assert!(
        tokio::time::timeout(WORKER_READY_POLL_INTERVAL, locks.lock(first))
            .await
            .is_err()
    );
}

#[tokio::test]
async fn explicit_resume_rejects_an_active_unfinalized_creation() {
    let directory = tempdir().unwrap();
    let host = Host::open(HostConfig::new(directory.path())).unwrap();
    let request = interleaving_request(directory.path(), Uuid::new_v4());
    let binding = reserve_interleaving(&host, &request);
    let error = host
        .resume_session(ResumeRequest {
            selection: TerminalLifecycleSelection::unconditional(binding.result.terminal_id),
            recreate: false,
        })
        .await
        .unwrap_err();
    assert_eq!(error.code, crate::protocol::error_code::CONFLICT);
    assert!(lock_recover(&host.catalog).list(false).unwrap().is_empty());
}

#[tokio::test]
async fn reconcile_never_recreates_a_worker_for_an_active_creation() {
    for previous_boot in [false, true] {
        let directory = tempdir().unwrap();
        let mut config = HostConfig::new(directory.path());
        config.runtime_executable = Some(directory.path().join("must-not-launch"));
        let host = Host::open(config).unwrap();
        let request = interleaving_request(directory.path(), Uuid::new_v4());
        let binding = reserve_interleaving(&host, &request);
        let manifest = host.prepare_bound_session(&binding).await.unwrap();
        let mut record = host_runtime_record(
            &host,
            manifest.terminal_id,
            directory.path().join("dead-worker.sock"),
        );
        record.worker_pid = u32::MAX;
        record.worker_start_time = MISSING_PROCESS_START_TIME;
        record.child_pid = u32::MAX;
        record.child_start_time = MISSING_PROCESS_START_TIME;
        if previous_boot {
            record.boot_id = ACTIVE_CREATION_PREVIOUS_BOOT.into();
        }
        atomic_write_json(&host.paths.runtime_record(manifest.terminal_id), &record).unwrap();

        let result = host.reconcile().await;
        assert_eq!(result.unreachable, 1);
        assert_eq!(result.needs_recreate, 0);
        assert!(!host.paths.worker_launch(manifest.terminal_id).exists());
        assert_eq!(
            lock_recover(&host.catalog)
                .get(manifest.terminal_id)
                .unwrap()
                .unwrap()
                .recovery_state,
            RecoveryState::Uncertain
        );
        assert!(
            lock_recover(&host.creation_capabilities)
                .active_creation(manifest.terminal_id)
                .unwrap()
                .is_some()
        );
    }
}

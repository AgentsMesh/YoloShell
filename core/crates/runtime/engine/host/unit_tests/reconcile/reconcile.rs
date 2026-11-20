use crate::host::tests::*;
use super::*;

#[tokio::test]
async fn catalog_archive_views_and_worker_unreachable_paths() {
    let directory = tempdir().unwrap();
    let host = Host::open(HostConfig::new(directory.path())).unwrap();
    let manifest = fixture_manifest(directory.path());
    host.paths.ensure_session(manifest.terminal_id).unwrap();
    host.catalog.lock().unwrap().insert(&manifest).unwrap();
    atomic_write_json(&host.paths.manifest(manifest.terminal_id), &manifest).unwrap();
    let view = host.view_for_manifest(manifest.clone()).await;
    assert!(view.runtime.is_none());
    let fetched = host
        .dispatch(&request(
            method::GET_SESSION,
            &TerminalRef {
                terminal_id: manifest.terminal_id,
            },
        ))
        .await;
    assert!(fetched.ok);
    let archived = host
        .dispatch(&request(
            method::ARCHIVE_SESSION,
            &TerminalRef {
                terminal_id: manifest.terminal_id,
            },
        ))
        .await;
    assert!(archived.ok);
    let missing_archive = host
        .dispatch(&request(
            method::ARCHIVE_SESSION,
            &TerminalRef {
                terminal_id: Uuid::new_v4(),
            },
        ))
        .await;
    assert_eq!(
        missing_archive.error.unwrap().code,
        crate::protocol::error_code::NOT_FOUND
    );
    let worker_request = host
        .dispatch(&request(
            method::TERMINATE_SESSION,
            &TerminalTerminateRequest {
                selection: TerminalLifecycleSelection::unconditional(manifest.terminal_id),
                force: false,
            },
        ))
        .await;
    assert!(worker_request.ok);
}

#[tokio::test]
async fn runtime_identity_and_reboot_reconciliation_paths_are_explicit() {
    let directory = tempdir().unwrap();
    let mut config = HostConfig::new(directory.path());
    config.runtime_executable = Some(directory.path().join("missing-runtime"));
    let host = Host::open(config).unwrap();
    for _ in [(), ()] {
        let manifest = fixture_manifest(directory.path());
        host.paths.ensure_session(manifest.terminal_id).unwrap();
        let record = RuntimeRecord {
            schema_version: RUNTIME_RECORD_SCHEMA_VERSION,
            terminal_id: manifest.terminal_id,
            runtime_epoch: Uuid::new_v4(),
            worker_id: Uuid::new_v4(),
            worker_pid: u32::MAX,
            worker_start_time: 0,
            child_pid: u32::MAX,
            child_start_time: 1,
            boot_id: "old-boot".into(),
            socket_path: host.paths.worker_socket(manifest.terminal_id).unwrap(),
            auth_token: "token".into(),
            controller_generation: 1,
            started_at_ms: now_ms(),
            command: manifest.command.clone(),
            args: Vec::new(),
            cwd: manifest.cwd.clone(),
            cols: 80,
            rows: 24,
            bootstrap_delivery: crate::model::BootstrapDelivery::NotRequired,
        };
        persist_host_session(&host, &manifest, &record);
        let error = host
            .live_runtime_record(manifest.terminal_id)
            .await
            .unwrap_err();
        assert_eq!(error.code, crate::protocol::error_code::HOST_REBOOTED);
    }
    let result = tokio::time::timeout(HOST_BLOCKING_TEST_TIMEOUT, host.reconcile())
        .await
        .expect("failed recovery must not deadlock the catalog");
    assert_eq!(result.unreachable, 2);
    assert_eq!(result.needs_recreate, 2);
    let states: Vec<_> = host
        .catalog
        .lock()
        .unwrap()
        .list(false)
        .unwrap()
        .into_iter()
        .map(|manifest| manifest.recovery_state)
        .collect();
    assert!(
        states
            .iter()
            .all(|state| *state == RecoveryState::NeedsRecreate)
    );
}

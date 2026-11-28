use crate::host::tests::*;
use super::*;

const RECONCILE_CONVERGENCE_SOCKET_EXTENSION: &str = "sock";
const RECONCILE_CONVERGENCE_SINGLE_RESULT: usize = 1;

#[tokio::test]
async fn convergence_empty_reconciliation_finishes_without_session_sweeps() {
    let directory = tempdir().unwrap();
    let host = Host::open(HostConfig::new(directory.path())).unwrap();
    assert_eq!(host.reconcile().await.unreachable, usize::default());
}

#[tokio::test]
async fn convergence_live_reconciliation_handles_starting_and_stable_states() {
    let directory = tempdir().unwrap();
    let host = Host::open(HostConfig::new(directory.path())).unwrap();
    let cases = [
        (TerminalRuntimeStatus::Starting, RecoveryState::Created),
        (TerminalRuntimeStatus::Running, RecoveryState::Reattached),
        (TerminalRuntimeStatus::Running, RecoveryState::Resumed),
        (TerminalRuntimeStatus::Finalizing, RecoveryState::Created),
    ];
    for (status, state) in cases {
        let mut manifest = fixture_manifest(directory.path());
        manifest.recovery_state = state;
        let socket = directory
            .path()
            .join(manifest.terminal_id.to_string())
            .with_extension(RECONCILE_CONVERGENCE_SOCKET_EXTENSION);
        let listener = UnixListener::bind(&socket).unwrap();
        let record = host_runtime_record(&host, manifest.terminal_id, socket);
        let mut view = status_view(&record, Some(Vec::new()));
        view.status = status;
        persist_host_session(&host, &manifest, &record);
        let server = tokio::spawn(fake_status_server(listener, record, Some(view)));

        let result = host.reconcile_manifest(manifest.clone()).await;
        if status == TerminalRuntimeStatus::Finalizing {
            assert_eq!(result.unreachable, RECONCILE_CONVERGENCE_SINGLE_RESULT);
        } else {
            assert_eq!(
                result.reattached, RECONCILE_CONVERGENCE_SINGLE_RESULT,
                "status {status:?}, state {state:?}"
            );
        }
        let stored = host
            .catalog_get(manifest.terminal_id)
            .await
            .unwrap()
            .unwrap();
        let expected = if state == RecoveryState::Created && status != TerminalRuntimeStatus::Finalizing {
            RecoveryState::Reattached
        } else {
            state
        };
        assert_eq!(stored.recovery_state, expected);
        server.await.unwrap();
    }
}

#[tokio::test]
async fn convergence_reconcile_rechecks_recoverability_after_restore_decision() {
    for decision in [
        AutomaticRestore::Eligible,
        AutomaticRestore::BlockedByRuntime,
    ] {
        let directory = tempdir().unwrap();
        let host = Arc::new(Host::open(HostConfig::new(directory.path())).unwrap());
        let manifest = fixture_manifest(directory.path());
        let socket = directory
            .path()
            .join(manifest.terminal_id.to_string())
            .with_extension(RECONCILE_CONVERGENCE_SOCKET_EXTENSION);
        let mut runtime = host_runtime_record(&host, manifest.terminal_id, socket);
        if decision == AutomaticRestore::Eligible {
            install_reaped_runtime_processes(&mut runtime);
        }
        persist_host_session(&host, &manifest, &runtime);
        if decision == AutomaticRestore::Eligible {
            host.secondary_custody
                .install_test_retirement_evidence(&host, &runtime);
        }
        let (reached, observed) = tokio::sync::oneshot::channel();
        let (release, released) = tokio::sync::oneshot::channel();
        *lock_recover(&host.catalog_test_probes.restore_decision) = Some(ReconcileRestoreProbe {
            reached,
            release: released,
        });
        let task_host = Arc::clone(&host);
        let task = tokio::spawn(async move { task_host.reconcile_manifest(manifest).await });
        assert_eq!(observed.await.unwrap(), decision);
        runtime.bootstrap_delivery = BootstrapDelivery::Pending;
        atomic_write_json(&host.paths.runtime_record(runtime.terminal_id), &runtime).unwrap();
        release.send(()).unwrap();

        let result = task.await.unwrap();
        assert_eq!(result.unreachable, RECONCILE_CONVERGENCE_SINGLE_RESULT);
        assert_eq!(result.needs_recreate, usize::default());
    }
}

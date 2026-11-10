use crate::host::tests::*;
use super::*;

const DURABLE_TERMINATION_SOCKET_NAME: &str = "durable-termination.sock";

#[tokio::test]
async fn durable_child_exit_skips_retired_worker_for_exact_termination() {
    let directory = tempdir().unwrap();
    let host = Host::open(HostConfig::new(directory.path())).unwrap();
    let manifest = fixture_manifest(directory.path());
    let runtime = lifecycle_convergence_dead_runtime(&host_runtime_record(
        &host,
        manifest.terminal_id,
        directory.path().join(DURABLE_TERMINATION_SOCKET_NAME),
    ));
    persist_host_session(&host, &manifest, &runtime);
    persist_archive_exit(&host, &runtime, runtime.runtime_epoch);
    host.secondary_custody.revoke_rpc_test(&runtime);
    let fence = LifecycleMutationFence {
        runtime: LifecycleRuntimeFence::Selected(runtime.clone()),
        selection: TerminalLifecycleSelection::at_runtime_epoch(manifest.terminal_id, runtime.runtime_epoch),
    };

    host.terminate_bound_session_with_fence(manifest.terminal_id, true, Some(&fence))
        .await
        .unwrap();
    assert_eq!(
        read_json::<RuntimeRecord>(&host.paths.runtime_record(manifest.terminal_id)).unwrap(),
        runtime
    );
    let tombstone: crate::model::TerminationTombstone =
        read_json(&host.paths.terminate_tombstone(manifest.terminal_id)).unwrap();
    assert_eq!(tombstone.runtime_epoch, runtime.runtime_epoch);
    assert!(tombstone.force);
}

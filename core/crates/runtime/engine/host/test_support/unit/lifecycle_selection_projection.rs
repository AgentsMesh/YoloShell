use crate::host::tests::*;
use super::*;

const PROJECTION_SOCKET: &str = "selection-projection.sock";

pub(super) async fn setup(dead: bool) -> (tempfile::TempDir, Host, TerminalManifest, RuntimeRecord) {
    let directory = tempdir().unwrap();
    let host = Host::open(HostConfig::new(directory.path())).unwrap();
    let mut manifest = fixture_manifest(directory.path());
    manifest.recovery_state = RecoveryState::Reattached;
    let mut runtime = host_runtime_record(
        &host,
        manifest.terminal_id,
        directory.path().join(PROJECTION_SOCKET),
    );
    if dead {
        let worker = reaped_test_process();
        runtime.worker_pid = worker.pid;
        runtime.worker_start_time = worker.start_time;
    }
    persist_host_session(&host, &manifest, &runtime);
    host.catalog_upsert(&manifest).await.unwrap();
    (directory, host, manifest, runtime)
}

pub(super) async fn selected(host: &Host, manifest: &TerminalManifest) -> TerminalLifecycleSelection {
    let view = host.view_for_manifest(manifest.clone()).await;
    let runtime = view.runtime.as_ref().unwrap();
    assert_eq!(view.manifest.recovery_state, RecoveryState::Uncertain);
    TerminalLifecycleSelection::selected(manifest.terminal_id, manifest.updated_at_ms, runtime.status)
        .with_expected_runtime_epoch(Some(runtime.runtime_epoch))
        .with_expected_manifest_sha256(Some(
            crate::model::TerminalManifestDigest::capture(&view.manifest).unwrap(),
        ))
}

pub(super) fn selection(manifest: &TerminalManifest, runtime: &RuntimeRecord) -> TerminalLifecycleSelection {
    TerminalLifecycleSelection::selected(
        manifest.terminal_id,
        manifest.updated_at_ms,
        TerminalRuntimeStatus::Unreachable,
    )
    .with_expected_runtime_epoch(Some(runtime.runtime_epoch))
    .with_expected_manifest_sha256(Some(
        crate::model::TerminalManifestDigest::capture(manifest).unwrap(),
    ))
}

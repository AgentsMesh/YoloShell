use crate::host::tests::*;
use super::*;

const RECOVERY_CATALOG_CONVERGENCE_DISPLACED: &str = "displaced-sessions";
const RECOVERY_CATALOG_CONVERGENCE_HOME: &str = "recovery-home";

#[tokio::test]
async fn convergence_recovery_persistence_stops_after_namespace_loss() {
    let root = tempdir().unwrap();
    let home = root.path().join(RECOVERY_CATALOG_CONVERGENCE_HOME);
    let host = Host::open(HostConfig::new(&home)).unwrap();
    let mut manifest = fixture_manifest(&home);
    seed_recovery_manifest(&host, &manifest).await;
    let expected = manifest_snapshot(&manifest);
    let sessions = host.paths.sessions_dir();
    std::fs::rename(
        &sessions,
        root.path().join(RECOVERY_CATALOG_CONVERGENCE_DISPLACED),
    )
    .unwrap();
    crate::paths::directory::PrivateDirectory::ensure(&sessions).unwrap();

    host.persist_recovery_state(&mut manifest, RecoveryState::Uncertain)
        .await;
    assert_manifest_snapshot(&manifest, &expected);
    host.persist_recovery_state_durable(&mut manifest, RecoveryState::NeedsRecreate);
    assert_manifest_snapshot(&manifest, &expected);
}

#[test]
fn convergence_empty_reconcile_probes_are_noops_and_publication_fences_identity() {
    let directory = tempdir().unwrap();
    let (started, _receiver) = tokio::sync::oneshot::channel();
    let mut scan = ReconcileScanProbe::started(started);
    scan.before_repair();
    scan.repair_completed(CatalogRepairProbeOutcome::Failed);

    let terminal_id = Uuid::new_v4();
    let (published, _receiver) = tokio::sync::oneshot::channel();
    let probe = RecoveryPublicationProbe::new(terminal_id, RecoveryState::Uncertain, published);
    let mut manifest = fixture_manifest(directory.path());
    manifest.terminal_id = terminal_id;
    manifest.recovery_state = RecoveryState::Uncertain;
    assert!(probe.matches(&manifest));
    manifest.terminal_id = Uuid::new_v4();
    assert!(!probe.matches(&manifest));
    manifest.terminal_id = terminal_id;
    manifest.recovery_state = RecoveryState::Created;
    assert!(!probe.matches(&manifest));
}

use super::super::*;
use crate::host::tests::*;

const HISTORY_ROWS: usize = 96;

#[tokio::test]
async fn watchdog_visits_only_reopened_sessions_after_history_settles() {
    let root = tempdir().unwrap();
    let host = Host::open(HostConfig::new(root.path())).unwrap();
    let mut history = Vec::new();
    for index in 0..HISTORY_ROWS {
        let mut manifest = fixture_manifest(root.path());
        manifest.name = format!("history-{index}");
        let id = manifest.terminal_id;
        let record = dead_archive_record(&host, id, root.path().join("old.sock"));
        if index % 2 == 0 {
            manifest.recovery_state = RecoveryState::NeedsRecreate;
        }
        persist_host_session(&host, &manifest, &record);
        if manifest.recovery_state != RecoveryState::NeedsRecreate {
            atomic_write_json(
                &host.paths.exit_record(id),
                &archive_exit_for(record.runtime_epoch),
            )
            .unwrap();
        }
        history.push(manifest);
    }
    host.reconcile_inner(false).await;
    assert!(host.recovery_queue.pending().is_empty());
    let (catalogue, mut queries) = tokio::sync::mpsc::unbounded_channel();
    *lock_recover(&host.catalog_test_probes.operations) = Some(catalogue);
    let (probe, mut scanned) = tokio::sync::oneshot::channel();
    *lock_recover(&host.catalog_test_probes.scan) = Some(ReconcileScanProbe::started(probe));
    host.reconcile_inner(false).await;
    assert!(matches!(
        queries.try_recv(),
        Err(tokio::sync::mpsc::error::TryRecvError::Empty)
    ));
    assert!(matches!(
        scanned.try_recv(),
        Err(tokio::sync::oneshot::error::TryRecvError::Empty)
    ));

    let reopened = history.pop().unwrap();
    host.catalog_upsert(&reopened).await.unwrap();
    assert_eq!(
        host.recovery_queue.pending()[0].terminal_id,
        reopened.terminal_id
    );
    // An unrelated historical row is corrupt. Only an explicit inventory repair visits it.
    corrupt_catalog_manifest(&host, history[0].terminal_id);
    assert_eq!(host.reconcile_inner(false).await.unreachable, 1);
    assert!(host.recovery_queue.pending().is_empty());
    assert!(host.catalog_get(history[0].terminal_id).await.is_err());
    host.reconcile().await;
    assert!(
        host.catalog_get(history[0].terminal_id)
            .await
            .unwrap()
            .is_some()
    );
}

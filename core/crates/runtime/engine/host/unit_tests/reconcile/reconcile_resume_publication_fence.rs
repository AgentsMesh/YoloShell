use crate::host::tests::*;
use super::*;

pub(super) struct ManifestPublicationPause {
    reached: std::sync::mpsc::Receiver<()>,
    release: std::sync::mpsc::Sender<()>,
}

impl ManifestPublicationPause {
    pub(super) async fn wait(self) -> std::sync::mpsc::Sender<()> {
        tokio::task::spawn_blocking(move || {
            self.reached
                .recv_timeout(HOST_BLOCKING_TEST_TIMEOUT)
                .unwrap();
            self.release
        })
        .await
        .unwrap()
    }
}

pub(super) fn pause_manifest_publication(host: &Host) -> ManifestPublicationPause {
    let (reached, observed) = std::sync::mpsc::channel();
    let (release, blocked) = std::sync::mpsc::channel();
    *lock_recover(&host.catalog_test_probes.manifest_publication_commit) =
        Some(Box::new(move || {
            reached.send(()).unwrap();
            blocked.recv_timeout(HOST_BLOCKING_TEST_TIMEOUT).unwrap();
            Ok(())
        }));
    ManifestPublicationPause {
        reached: observed,
        release,
    }
}

fn recovery_manifest(original: &TerminalManifest, state: RecoveryState) -> TerminalManifest {
    let mut candidate = original.clone();
    candidate.recovery_state = state;
    candidate.updated_at_ms = now_ms();
    candidate
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn terminal_confirmation_rolls_back_when_manifest_drifts_at_commit() {
    let directory = tempdir().unwrap();
    let host = Arc::new(Host::open(HostConfig::new(directory.path())).unwrap());
    let indexed = fixture_manifest(directory.path());
    seed_recovery_manifest(&host, &indexed).await;
    let terminal = recovery_manifest(&indexed, RecoveryState::NeedsRecreate);
    host.persist_recovery_manifest(&indexed, &terminal).unwrap();
    let pause = pause_manifest_publication(&host);
    let terminal_id = indexed.terminal_id;
    let task_host = Arc::clone(&host);
    let publication =
        tokio::spawn(async move { task_host.confirm_terminal_resume_failure(terminal_id).await });

    let release = pause.wait().await;
    let replacement = recovery_manifest(&terminal, RecoveryState::Uncertain);
    host.persist_recovery_manifest(&terminal, &replacement)
        .unwrap();
    release.send(()).unwrap();

    assert!(publication.await.unwrap().is_err());
    let durable: TerminalManifest = read_json(&host.paths.manifest(terminal_id)).unwrap();
    assert_manifest_snapshot(&durable, &manifest_snapshot(&replacement));
    assert_catalog_and_wire_snapshot(&host, terminal_id, &manifest_snapshot(&indexed)).await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn automatic_block_rolls_back_when_manifest_drifts_after_cas() {
    let directory = tempdir().unwrap();
    let host = Arc::new(Host::open(HostConfig::new(directory.path())).unwrap());
    let indexed = fixture_manifest(directory.path());
    seed_recovery_manifest(&host, &indexed).await;
    let pause = pause_manifest_publication(&host);
    let terminal_id = indexed.terminal_id;
    let task_host = Arc::clone(&host);
    let publication =
        tokio::spawn(async move { task_host.converge_automatic_resume_block(terminal_id).await });

    let release = pause.wait().await;
    let installed: TerminalManifest = read_json(&host.paths.manifest(terminal_id)).unwrap();
    assert_eq!(installed.recovery_state, RecoveryState::Uncertain);
    let replacement = recovery_manifest(&installed, RecoveryState::Reattached);
    host.persist_recovery_manifest(&installed, &replacement)
        .unwrap();
    release.send(()).unwrap();

    assert!(publication.await.unwrap().is_err());
    let durable: TerminalManifest = read_json(&host.paths.manifest(terminal_id)).unwrap();
    assert_manifest_snapshot(&durable, &manifest_snapshot(&replacement));
    assert_catalog_and_wire_snapshot(&host, terminal_id, &manifest_snapshot(&indexed)).await;
}

#[tokio::test]
async fn exact_terminal_manifest_identity_publishes_to_catalog() {
    let directory = tempdir().unwrap();
    let host = Host::open(HostConfig::new(directory.path())).unwrap();
    let indexed = fixture_manifest(directory.path());
    seed_recovery_manifest(&host, &indexed).await;
    let terminal = recovery_manifest(&indexed, RecoveryState::NeedsRecreate);
    host.persist_recovery_manifest(&indexed, &terminal).unwrap();

    host.confirm_terminal_resume_failure(indexed.terminal_id)
        .await
        .unwrap();

    assert_catalog_and_wire_snapshot(&host, indexed.terminal_id, &manifest_snapshot(&terminal))
        .await;
}

#[tokio::test]
async fn exact_automatic_block_cas_publishes_to_catalog() {
    let directory = tempdir().unwrap();
    let host = Host::open(HostConfig::new(directory.path())).unwrap();
    let indexed = fixture_manifest(directory.path());
    seed_recovery_manifest(&host, &indexed).await;

    host.converge_automatic_resume_block(indexed.terminal_id)
        .await
        .unwrap();

    let durable: TerminalManifest = read_json(&host.paths.manifest(indexed.terminal_id)).unwrap();
    assert_eq!(durable.recovery_state, RecoveryState::Uncertain);
    assert_catalog_and_wire_snapshot(&host, indexed.terminal_id, &manifest_snapshot(&durable)).await;
}

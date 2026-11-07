use crate::host::tests::*;
type RecoveryManifestTestPublication =
    crate::host::recovery_manifest_publication::PreparedRecoveryManifest;

const RECOVERY_PUBLICATION_TIMESTAMP_STEP: i64 = 1;
const RECOVERY_PUBLICATION_GENERATION_STEP: u64 = 1;
const RECOVERY_PUBLICATION_REPLACEMENT_NAME: &str = "replacement-recovery-manifest";
const RECOVERY_PUBLICATION_REPLACEMENT_DIRECTORY: &str = "retained-recovery-session";
const RECOVERY_PUBLICATION_LOCK_NOT_OBSERVED: &str =
    "Host recovery publication did not wait for the session directory guard";
const RECOVERY_PUBLICATION_BYPASSED_WORKER: &str =
    "Host recovery publication completed while Worker held the directory guard";

async fn recovery_manifest_fixture() -> (tempfile::TempDir, Arc<Host>, TerminalManifest) {
    let directory = tempdir().unwrap();
    let host = Arc::new(Host::open(HostConfig::new(directory.path())).unwrap());
    let manifest = fixture_manifest(directory.path());
    seed_recovery_manifest(&host, &manifest).await;
    (directory, host, manifest)
}

fn recovery_manifest_candidate(original: &TerminalManifest) -> TerminalManifest {
    let mut candidate = original.clone();
    candidate.recovery_state = RecoveryState::Uncertain;
    candidate.updated_at_ms += RECOVERY_PUBLICATION_TIMESTAMP_STEP;
    candidate
}

fn wait_for_recovery_directory_waiter(
    identity: crate::paths::identity::ObjectIdentity,
    completed: impl Fn() -> bool,
) -> Result<()> {
    let deadline = std::time::Instant::now() + HOST_BLOCKING_TEST_TIMEOUT;
    while crate::paths::test_support::queued_process_lock_waiters(identity) == usize::default() {
        anyhow::ensure!(!completed(), RECOVERY_PUBLICATION_BYPASSED_WORKER);
        anyhow::ensure!(
            std::time::Instant::now() < deadline,
            RECOVERY_PUBLICATION_LOCK_NOT_OBSERVED
        );
        std::thread::yield_now();
    }
    Ok(())
}

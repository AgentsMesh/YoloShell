use crate::host::tests::*;
use super::*;

const PRODUCT_TERMINATE_NAME: &str = "product-terminate";
const PRODUCT_ARCHIVE_NAME: &str = "product-archive";
const PRODUCT_DISCARD_NAME: &str = "product-discard";
const PRODUCT_ARCHIVED_NAME: &str = "already-archived-product";

pub(super) async fn lifecycle_product_fixture(
    name: &str,
    phase: CreationPhase,
) -> (
    tempfile::TempDir,
    Host,
    CreateTerminal,
    CreationBinding,
    TerminalManifest,
    RuntimeRecord,
    PathBuf,
) {
    let directory = tempdir().unwrap();
    let host = Host::open(HostConfig::new(directory.path())).unwrap();
    let (request, mut binding, path) = reserve_test_creation(&host, directory.path(), name);
    let manifest = host.prepare_bound_session(&binding).await.unwrap();
    let launch = lock_recover(&host.creation_capabilities)
        .prepare_launch(&path, binding.result)
        .unwrap();
    binding = CreationLedger::open(&path)
        .unwrap()
        .bindings
        .into_iter()
        .next()
        .unwrap();
    assert_eq!(binding.launch.as_ref(), Some(&launch));
    let worker_launch = planned_worker_launch(&host, &binding, binding.launch.as_ref().unwrap());
    let runtime =
        runtime_for_creation(&host, &worker_launch, BootstrapDelivery::NotRequired, false);
    atomic_write_json(
        &host.paths.worker_launch(binding.result.terminal_id),
        &worker_launch,
    )
    .unwrap();
    persist_archive_runtime(&host, &runtime);
    persist_archive_exit(&host, &runtime, runtime.runtime_epoch);
    for next in [CreationPhase::Ready, phase] {
        lock_recover(&host.creation_capabilities)
            .set_active_phase(&path, binding.result, next)
            .unwrap();
    }
    binding.phase = phase;
    assert_eq!(
        host.creation_runtime_evidence(&binding, binding.launch.as_ref().unwrap()),
        CreationRuntimeEvidence::Ready
    );
    (directory, host, request, binding, manifest, runtime, path)
}

fn assert_product_tombstone(
    tombstone: &crate::model::TerminationTombstone,
    binding: &CreationBinding,
    runtime: &RuntimeRecord,
) {
    let launch = binding.launch.as_ref().unwrap();
    assert_eq!(runtime.runtime_epoch, launch.runtime_epoch);
    assert_eq!(runtime.worker_id, launch.worker_id);
    assert_eq!(tombstone.runtime_epoch, runtime.runtime_epoch);
    assert_eq!(runtime.worker_pid, MISSING_CREATION_WORKER_ID);
    assert_eq!(runtime.child_pid, MISSING_CREATION_CHILD_ID);
    assert_eq!(
        tombstone.validate_for(binding.result.terminal_id, launch.runtime_epoch),
        Ok(())
    );
    assert!(tombstone.force);
}

#[tokio::test]
async fn ordinary_termination_persists_each_active_phase_as_terminated() {
    for (initial, expects_termination) in [
        (CreationPhase::Ready, true),
        (CreationPhase::Terminated, false),
    ] {
        let (_directory, host, _request, binding, _manifest, runtime, path) =
            lifecycle_product_fixture(PRODUCT_TERMINATE_NAME, initial).await;
        host.terminate_session(TerminationIntent {
            terminal_id: binding.result.terminal_id,
            force: true,
        })
        .await
        .unwrap();
        let durable = CreationLedger::open(&path).unwrap();
        assert_eq!(
            durable.bindings.first().unwrap().phase,
            CreationPhase::Terminated
        );
        let tombstone_path = host.paths.terminate_tombstone(binding.result.terminal_id);
        assert_eq!(tombstone_path.try_exists().unwrap(), expects_termination);
        if expects_termination {
            let tombstone: crate::model::TerminationTombstone = read_json(&tombstone_path).unwrap();
            assert_product_tombstone(&tombstone, &binding, &runtime);
        }
        assert!(
            host.creation_receipts
                .lookup_binding(&binding)
                .unwrap()
                .is_none()
        );
    }
}

#[tokio::test]
async fn ordinary_archive_completes_each_permitted_phase_with_archived_receipt() {
    for phase in [CreationPhase::Ready, CreationPhase::Terminated] {
        let (_directory, host, _request, binding, _manifest, _runtime, path) =
            lifecycle_product_fixture(PRODUCT_ARCHIVE_NAME, phase).await;
        let view = host
            .archive_session(TerminalRef {
                terminal_id: binding.result.terminal_id,
            })
            .await
            .unwrap();
        assert!(view.manifest.archived);
        assert_transition_ledger(&path, &[]);
        assert_eq!(
            host.creation_receipts
                .lookup_binding(&binding)
                .unwrap()
                .unwrap()
                .completion,
            CreationCompletion::Archived
        );
    }
}

#[tokio::test]
async fn ordinary_discard_terminates_archives_and_retires_active_product() {
    let (_directory, host, _request, binding, _manifest, runtime, path) =
        lifecycle_product_fixture(PRODUCT_DISCARD_NAME, CreationPhase::Ready).await;
    let view = host
        .discard_session(TerminationIntent {
            terminal_id: binding.result.terminal_id,
            force: true,
        })
        .await
        .unwrap();
    assert!(view.manifest.archived);
    assert_transition_ledger(&path, &[]);
    assert_eq!(
        host.creation_receipts
            .lookup_binding(&binding)
            .unwrap()
            .unwrap()
            .completion,
        CreationCompletion::Archived
    );
    let tombstone: crate::model::TerminationTombstone =
        read_json(&host.paths.terminate_tombstone(binding.result.terminal_id)).unwrap();
    assert_product_tombstone(&tombstone, &binding, &runtime);
}

#[tokio::test]
async fn ordinary_archive_without_active_creation_does_not_forge_receipt() {
    let directory = tempdir().unwrap();
    let host = Host::open(HostConfig::new(directory.path())).unwrap();
    let mut manifest = fixture_manifest(directory.path());
    manifest.name = PRODUCT_ARCHIVED_NAME.into();
    manifest.archived = true;
    seed_recovery_manifest(&host, &manifest).await;
    let view = host
        .archive_session(TerminalRef {
            terminal_id: manifest.terminal_id,
        })
        .await
        .unwrap();
    assert!(view.manifest.archived);
    assert!(
        lock_recover(&host.creation_capabilities)
            .bindings
            .is_empty()
    );
    assert!(
        host.creation_receipts
            .lookup_session(manifest.terminal_id)
            .unwrap()
            .is_none()
    );
}

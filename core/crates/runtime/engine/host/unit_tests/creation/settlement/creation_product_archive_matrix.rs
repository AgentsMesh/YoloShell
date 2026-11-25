use crate::host::tests::*;
use super::*;

const PRODUCT_ARCHIVE_MATRIX_NAME: &str = "product-archive-matrix";
pub(super) const PRODUCT_ARCHIVE_BARRIER_NAME: &str = "product-archive-barrier";
const PRODUCT_ARCHIVE_MISSING_RUNTIME: &str = "missing-product-runtime";

fn product_archive_binding_phase(binding: &mut CreationBinding, phase: CreationPhase) {
    binding.phase = phase;
    if phase == CreationPhase::Reserved {
        binding.launch_generation = RESERVED_CREATION_LAUNCH_GENERATION;
        binding.launch = None;
    }
}

#[test]
fn product_archive_completion_enforces_the_complete_phase_matrix() {
    for (phase, permitted) in [
        (CreationPhase::Reserved, false),
        (CreationPhase::Launching, false),
        (CreationPhase::SpawnFailed, false),
        (CreationPhase::Ready, true),
        (CreationPhase::Terminated, true),
    ] {
        let (_directory, store, mut binding, path) = receipt_test_setup();
        product_archive_binding_phase(&mut binding, phase);
        let mut ledger = transition_ledger(&path, &binding);
        let result = ledger.complete_product_archive(&path, binding.result, &store);
        if permitted {
            result.unwrap();
            assert_transition_ledger(&path, &[]);
            assert_eq!(
                store.lookup_binding(&binding).unwrap().unwrap().completion,
                CreationCompletion::Archived
            );
        } else {
            assert_eq!(
                result.unwrap_err().code,
                crate::protocol::error_code::CONFLICT
            );
            assert_transition_ledger(&path, std::slice::from_ref(&binding));
            assert_receipt_indexes_empty(&store);
        }
    }
}

async fn pre_ready_product_archive_fixture(
    phase: CreationPhase,
) -> (
    tempfile::TempDir,
    Host,
    CreateTerminal,
    CreationBinding,
    TerminalManifest,
    PathBuf,
) {
    let directory = tempdir().unwrap();
    let mut config = HostConfig::new(directory.path());
    config.runtime_executable = Some(directory.path().join(PRODUCT_ARCHIVE_MISSING_RUNTIME));
    let host = Host::open(config).unwrap();
    let (request, reserved, path) =
        reserve_test_creation(&host, directory.path(), PRODUCT_ARCHIVE_MATRIX_NAME);
    let manifest = host.prepare_bound_session(&reserved).await.unwrap();
    if phase != CreationPhase::Reserved {
        let mut ledger = lock_recover(&host.creation_capabilities);
        ledger.prepare_launch(&path, reserved.result).unwrap();
        if phase == CreationPhase::SpawnFailed {
            ledger
                .set_active_phase(&path, reserved.result, CreationPhase::SpawnFailed)
                .unwrap();
        }
    }
    let binding = CreationLedger::open(&path)
        .unwrap()
        .bindings
        .into_iter()
        .next()
        .unwrap();
    assert_eq!(binding.phase, phase);
    (directory, host, request, binding, manifest, path)
}

#[tokio::test]
async fn ordinary_archive_rejects_every_pre_ready_phase_before_side_effects() {
    for phase in [
        CreationPhase::Reserved,
        CreationPhase::Launching,
        CreationPhase::SpawnFailed,
    ] {
        let (_directory, host, _request, binding, manifest, path) =
            pre_ready_product_archive_fixture(phase).await;
        assert_pre_ready_archive_is_side_effect_free(&host, &binding, &manifest, &path).await;
    }
}

async fn assert_pre_ready_archive_is_side_effect_free(
    host: &Host,
    binding: &CreationBinding,
    manifest: &TerminalManifest,
    path: &Path,
) {
    let terminal_id = binding.result.terminal_id;
    let manifest_before = std::fs::read(host.paths.manifest(terminal_id)).unwrap();
    let ledger_before = CreationLedger::open(path).unwrap().bindings;
    assert_archive_runtime_evidence_missing(host, terminal_id);
    let error = host
        .archive_session(TerminalRef { terminal_id })
        .await
        .unwrap_err();
    assert_eq!(error.code, crate::protocol::error_code::CONFLICT);
    assert_archive_runtime_evidence_missing(host, terminal_id);
    assert_eq!(
        std::fs::read(host.paths.manifest(terminal_id)).unwrap(),
        manifest_before
    );
    assert_manifest_snapshot(
        &host.catalog_get(terminal_id).await.unwrap().unwrap(),
        &manifest_snapshot(manifest),
    );
    assert_eq!(CreationLedger::open(path).unwrap().bindings, ledger_before);
    assert!(
        host.creation_receipts
            .lookup_binding(binding)
            .unwrap()
            .is_none()
    );
    assert_receipt_indexes_empty(&host.creation_receipts);
}

fn assert_archive_runtime_evidence_missing(host: &Host, terminal_id: Uuid) {
    for evidence in [
        host.paths.worker_launch(terminal_id),
        host.paths.runtime_record(terminal_id),
        host.paths.exit_record(terminal_id),
    ] {
        assert!(!evidence.try_exists().unwrap());
    }
}

#[tokio::test]
async fn launching_archive_barrier_preserves_the_original_creation_transaction() {
    let (_directory, host, request, binding, manifest, path) =
        pre_ready_product_archive_fixture(CreationPhase::Launching).await;
    assert_pre_ready_archive_is_side_effect_free(&host, &binding, &manifest, &path).await;
    let planned = binding.launch.as_ref().unwrap();
    let launch = planned_worker_launch(&host, &binding, planned);
    let runtime = runtime_for_creation(&host, &launch, BootstrapDelivery::NotRequired, true);
    atomic_write_json(
        &host.paths.worker_launch(binding.result.terminal_id),
        &launch,
    )
    .unwrap();
    atomic_write_json(
        &host.paths.runtime_record(binding.result.terminal_id),
        &runtime,
    )
    .unwrap();
    let authentication = spawn_creation_authentication_server(&runtime);

    assert_eq!(host.create_session(request).await.unwrap(), binding.result);
    authentication.await.unwrap();
    let settled = CreationLedger::open(&path)
        .unwrap()
        .bindings
        .into_iter()
        .next()
        .unwrap();
    assert_eq!(settled.phase, CreationPhase::Ready);
    assert_eq!(settled.launch_generation, binding.launch_generation);
    assert_eq!(settled.launch, binding.launch);
    assert!(
        host.creation_receipts
            .lookup_binding(&binding)
            .unwrap()
            .is_none()
    );
}

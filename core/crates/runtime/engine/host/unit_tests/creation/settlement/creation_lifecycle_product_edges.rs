use crate::host::tests::*;
use super::*;

const PRODUCT_TERMINATED_DISCARD_NAME: &str = "terminated-product-discard";
const PRODUCT_INVALID_DISCARD_NAME: &str = "invalid-product-discard";

#[tokio::test]
async fn ordinary_discard_archives_terminated_creation_without_reterminating() {
    let (_directory, host, _request, binding, _manifest, _runtime, path) =
        lifecycle_product_fixture(PRODUCT_TERMINATED_DISCARD_NAME, CreationPhase::Terminated).await;
    let tombstone = host.paths.terminate_tombstone(binding.result.terminal_id);
    assert!(!tombstone.try_exists().unwrap());

    let view = host
        .discard_session(TerminationIntent {
            terminal_id: binding.result.terminal_id,
            force: true,
        })
        .await
        .unwrap();

    assert!(view.manifest.archived);
    assert!(!tombstone.try_exists().unwrap());
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

#[tokio::test]
async fn ordinary_discard_rejects_every_pre_ready_creation_without_side_effects() {
    for phase in [
        CreationPhase::Reserved,
        CreationPhase::Launching,
        CreationPhase::SpawnFailed,
    ] {
        let directory = tempdir().unwrap();
        let host = Host::open(HostConfig::new(directory.path())).unwrap();
        let (_, binding, path) =
            reserve_test_creation(&host, directory.path(), PRODUCT_INVALID_DISCARD_NAME);
        if phase != CreationPhase::Reserved {
            let mut ledger = lock_recover(&host.creation_capabilities);
            ledger.prepare_launch(&path, binding.result).unwrap();
            if phase == CreationPhase::SpawnFailed {
                ledger
                    .set_active_phase(&path, binding.result, CreationPhase::SpawnFailed)
                    .unwrap();
            }
        }
        let error = host
            .discard_session(TerminationIntent {
                terminal_id: binding.result.terminal_id,
                force: true,
            })
            .await
            .unwrap_err();

        assert_eq!(error.code, crate::protocol::error_code::CONFLICT);
        assert_eq!(
            CreationLedger::open(&path)
                .unwrap()
                .bindings
                .first()
                .unwrap()
                .phase,
            phase
        );
        assert!(
            !host
                .paths
                .terminate_tombstone(binding.result.terminal_id)
                .try_exists()
                .unwrap()
        );
        assert!(
            host.creation_receipts
                .lookup_binding(&binding)
                .unwrap()
                .is_none()
        );
    }
}

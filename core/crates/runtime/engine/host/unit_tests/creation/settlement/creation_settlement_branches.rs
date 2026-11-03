use crate::host::tests::*;
use super::*;

const RESERVED_SETTLEMENT_NAME: &str = "reserved-settlement";

#[derive(Clone, Copy, Debug)]
enum SettlementCase {
    Missing,
    Ready,
    UnauthenticatedReady,
    LaunchFailure,
    SpawnFailedReplay,
    Pending,
    Mismatch,
}

#[tokio::test]
async fn reserved_creation_requires_launch_without_durable_mutation() {
    let directory = tempdir().unwrap();
    let host = Host::open(HostConfig::new(directory.path())).unwrap();
    let (_, binding, path) =
        reserve_test_creation(&host, directory.path(), RESERVED_SETTLEMENT_NAME);
    assert!(
        host.creation_requires_launch(&path, &binding)
            .await
            .unwrap()
    );
    assert_transition_ledger(&path, &[binding]);
}

#[tokio::test]
async fn launch_settlement_matrix_commits_only_proven_terminal_evidence() {
    let cases = [
        SettlementCase::Missing,
        SettlementCase::Ready,
        SettlementCase::UnauthenticatedReady,
        SettlementCase::LaunchFailure,
        SettlementCase::SpawnFailedReplay,
        SettlementCase::Pending,
        SettlementCase::Mismatch,
    ];
    for case in cases {
        let runtime_live = matches!(
            case,
            SettlementCase::Ready | SettlementCase::UnauthenticatedReady | SettlementCase::Pending
        );
        let (_directory, mut host, _, mut binding, planned, mut runtime) =
            pending_creation_fixture(runtime_live).await;
        if matches!(
            case,
            SettlementCase::Ready
                | SettlementCase::LaunchFailure
                | SettlementCase::SpawnFailedReplay
        ) {
            grant_terminal_creation_settlement_budget(&mut host);
        }
        let path = host.paths.home().join(CREATION_LEDGER_FILE_NAME);
        binding.phase = CreationPhase::Launching;
        binding.launch = Some(planned.clone());
        let mut authentication = None;
        match case {
            SettlementCase::Missing => {
                std::fs::remove_file(host.paths.worker_launch(binding.result.terminal_id)).unwrap();
                std::fs::remove_file(host.paths.runtime_record(binding.result.terminal_id)).unwrap();
            }
            SettlementCase::Ready | SettlementCase::UnauthenticatedReady => {
                runtime.bootstrap_delivery = BootstrapDelivery::NotRequired;
                atomic_write_json(
                    &host.paths.runtime_record(binding.result.terminal_id),
                    &runtime,
                )
                .unwrap();
                if matches!(case, SettlementCase::Ready) {
                    authentication = Some(spawn_creation_authentication_server(&runtime));
                }
            }
            SettlementCase::LaunchFailure | SettlementCase::SpawnFailedReplay => {
                runtime.bootstrap_delivery = BootstrapDelivery::Failed;
                atomic_write_json(
                    &host.paths.runtime_record(binding.result.terminal_id),
                    &runtime,
                )
                .unwrap();
                if matches!(case, SettlementCase::SpawnFailedReplay) {
                    lock_recover(&host.creation_capabilities)
                        .set_active_phase(&path, binding.result, CreationPhase::SpawnFailed)
                        .unwrap();
                    binding.phase = CreationPhase::SpawnFailed;
                }
            }
            SettlementCase::Pending => {}
            SettlementCase::Mismatch => {
                runtime.auth_token = Uuid::new_v4().simple().to_string();
                atomic_write_json(
                    &host.paths.runtime_record(binding.result.terminal_id),
                    &runtime,
                )
                .unwrap();
            }
        }

        let result = host.creation_requires_launch(&path, &binding).await;
        if let Some(authentication) = authentication {
            authentication.await.unwrap();
        }
        let expected_phase = match case {
            SettlementCase::Missing => {
                assert!(result.unwrap());
                CreationPhase::Launching
            }
            SettlementCase::Ready => {
                assert!(!result.unwrap());
                CreationPhase::Ready
            }
            SettlementCase::UnauthenticatedReady => {
                assert_eq!(
                    result.unwrap_err().code,
                    crate::protocol::error_code::DELIVERY_UNKNOWN
                );
                CreationPhase::Launching
            }
            SettlementCase::LaunchFailure => {
                assert_eq!(
                    result.unwrap_err().code,
                    crate::protocol::error_code::WORKER_START_FAILED
                );
                CreationPhase::SpawnFailed
            }
            SettlementCase::SpawnFailedReplay => {
                assert!(result.unwrap());
                CreationPhase::SpawnFailed
            }
            SettlementCase::Pending => {
                assert_eq!(
                    result.unwrap_err().code,
                    crate::protocol::error_code::DELIVERY_UNKNOWN
                );
                CreationPhase::Launching
            }
            SettlementCase::Mismatch => {
                assert_eq!(
                    result.unwrap_err().code,
                    crate::protocol::error_code::IDENTITY_MISMATCH
                );
                CreationPhase::Launching
            }
        };
        let durable = CreationLedger::open(&path).unwrap();
        assert_eq!(durable.bindings[0].phase, expected_phase, "{case:?}");
        assert_eq!(durable.bindings[0].launch.as_ref(), Some(&planned));
    }
}

#[tokio::test]
async fn ready_creation_requires_matching_manifest_and_runtime() {
    let (_directory, host, _, mut binding, planned, mut runtime) =
        pending_creation_fixture(false).await;
    let path = host.paths.home().join(CREATION_LEDGER_FILE_NAME);
    runtime.bootstrap_delivery = BootstrapDelivery::NotRequired;
    atomic_write_json(
        &host.paths.runtime_record(binding.result.terminal_id),
        &runtime,
    )
    .unwrap();
    lock_recover(&host.creation_capabilities)
        .set_active_phase(&path, binding.result, CreationPhase::Ready)
        .unwrap();
    binding.phase = CreationPhase::Ready;
    binding.launch = Some(planned);
    assert!(
        !host
            .creation_requires_launch(&path, &binding)
            .await
            .unwrap()
    );

    std::fs::remove_file(host.paths.manifest(binding.result.terminal_id)).unwrap();
    lock_recover(&host.catalog)
        .delete(binding.result.terminal_id)
        .unwrap();
    let error = host
        .creation_requires_launch(&path, &binding)
        .await
        .unwrap_err();
    assert_eq!(error.code, crate::protocol::error_code::INTERNAL);
    assert_transition_ledger(&path, &[binding]);
}

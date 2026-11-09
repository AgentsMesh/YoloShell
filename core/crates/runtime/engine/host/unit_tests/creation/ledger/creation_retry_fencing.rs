use crate::host::tests::*;
use super::*;

const RETRY_FENCING_NAME: &str = "retry-fencing";
const RETRY_FENCING_MISSING_RUNTIME: &str = "missing-retry-runtime";
const RETRIED_CREATION_LAUNCH_GENERATION: u64 =
    INITIAL_CREATION_LAUNCH_GENERATION + INITIAL_CREATION_LAUNCH_GENERATION;

async fn failed_creation_for_retry() -> (
    tempfile::TempDir,
    Host,
    CreateTerminal,
    CreationBinding,
    CreationLaunchIdentity,
    RuntimeRecord,
    PathBuf,
) {
    let directory = tempdir().unwrap();
    let mut config = HostConfig::new(directory.path());
    config.runtime_executable = Some(directory.path().join(RETRY_FENCING_MISSING_RUNTIME));
    let host = Host::open(config).unwrap();
    let (request, reserved, ledger_path) =
        reserve_test_creation(&host, directory.path(), RETRY_FENCING_NAME);
    host.prepare_bound_session(&reserved).await.unwrap();
    let previous = lock_recover(&host.creation_capabilities)
        .prepare_launch(&ledger_path, reserved.result)
        .unwrap();
    let launch = planned_worker_launch(&host, &reserved, &previous);
    let runtime = runtime_for_creation(&host, &launch, BootstrapDelivery::Failed, false);
    atomic_write_json(
        &host.paths.worker_launch(reserved.result.terminal_id),
        &launch,
    )
    .unwrap();
    atomic_write_json(
        &host.paths.runtime_record(reserved.result.terminal_id),
        &runtime,
    )
    .unwrap();
    lock_recover(&host.creation_capabilities)
        .set_active_phase(&ledger_path, reserved.result, CreationPhase::SpawnFailed)
        .unwrap();
    let failed = lock_recover(&host.creation_capabilities)
        .bindings
        .iter()
        .find(|binding| binding.result == reserved.result)
        .unwrap()
        .clone();
    (
        directory,
        host,
        request,
        failed,
        previous,
        runtime,
        ledger_path,
    )
}

pub(super) fn assert_rotated_launch_authority(
    previous: &CreationLaunchIdentity,
    current: &CreationLaunchIdentity,
) {
    assert_ne!(current.runtime_epoch, previous.runtime_epoch);
    assert_ne!(current.worker_id, previous.worker_id);
    assert_ne!(current.auth_token, previous.auth_token);
}

#[tokio::test]
async fn retry_ledger_fence_ignores_previous_launch_and_failed_runtime() {
    let (_directory, host, _request, failed, previous, runtime, ledger_path) =
        failed_creation_for_retry().await;
    let (attempt, current) = lock_recover(&host.creation_capabilities)
        .prepare_launch_attempt(&ledger_path, failed.result)
        .unwrap();

    assert_eq!(attempt.phase, CreationPhase::Launching);
    assert_eq!(
        attempt.launch_generation,
        RETRIED_CREATION_LAUNCH_GENERATION
    );
    assert_rotated_launch_authority(&previous, &current);
    assert_eq!(
        host.creation_runtime_evidence(&attempt, &current),
        CreationRuntimeEvidence::Missing
    );

    let current_launch = planned_worker_launch(&host, &attempt, &current);
    atomic_write_json(
        &host.paths.worker_launch(attempt.result.terminal_id),
        &current_launch,
    )
    .unwrap();
    assert_eq!(
        host.creation_runtime_evidence(&attempt, &current),
        CreationRuntimeEvidence::Unknown
    );
    assert_eq!(
        read_json::<RuntimeRecord>(&host.paths.runtime_record(attempt.result.terminal_id)).unwrap(),
        runtime
    );
}

#[tokio::test]
async fn full_spawn_failed_retry_cannot_be_decided_by_previous_failed_runtime() {
    let (_directory, host, request, failed, previous, runtime, ledger_path) =
        failed_creation_for_retry().await;

    let error = host.create_session(request).await.unwrap_err();
    assert_eq!(error.code, crate::protocol::error_code::WORKER_START_FAILED);
    let durable = CreationLedger::open(&ledger_path).unwrap();
    let retried = durable
        .bindings
        .iter()
        .find(|binding| binding.result == failed.result)
        .unwrap();
    let current = retried.launch.as_ref().unwrap();
    assert_eq!(retried.phase, CreationPhase::SpawnFailed);
    assert_eq!(
        retried.launch_generation,
        RETRIED_CREATION_LAUNCH_GENERATION
    );
    assert_rotated_launch_authority(&previous, current);
    assert_eq!(
        host.creation_runtime_evidence(retried, current),
        CreationRuntimeEvidence::Missing
    );
    assert_eq!(
        read_json::<RuntimeRecord>(&host.paths.runtime_record(failed.result.terminal_id)).unwrap(),
        runtime
    );
}

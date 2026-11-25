use super::*;
use crate::host::tests::*;

const CREATION_RUNTIME_PREVIOUS_BOOT: &str = "creation-runtime-previous-boot";
const CREATION_RUNTIME_CURRENT_BOOT: &str = "creation-runtime-current-boot";

#[tokio::test]
async fn creation_runtime_evidence_distinguishes_durable_file_permutations() {
    let (_directory, host, _, binding, planned, mut runtime) =
        pending_creation_fixture(false).await;
    let launch_path = host.paths.worker_launch(binding.result.terminal_id);
    let runtime_path = host.paths.runtime_record(binding.result.terminal_id);

    std::fs::remove_file(&runtime_path).unwrap();
    assert_eq!(
        host.creation_runtime_evidence(&binding, &planned),
        CreationRuntimeEvidence::Unknown
    );
    std::fs::remove_file(&launch_path).unwrap();
    assert_eq!(
        host.creation_runtime_evidence(&binding, &planned),
        CreationRuntimeEvidence::Missing
    );

    atomic_write_json(&runtime_path, &runtime).unwrap();
    assert_eq!(
        host.creation_runtime_evidence(&binding, &planned),
        CreationRuntimeEvidence::Failed(CreationFailureResolution::Resolved)
    );
    runtime.worker_pid = std::process::id();
    runtime.worker_start_time = process::process_start_time(runtime.worker_pid).unwrap();
    atomic_write_json(&runtime_path, &runtime).unwrap();
    assert_eq!(
        host.creation_runtime_evidence(&binding, &planned),
        CreationRuntimeEvidence::MatchingPending
    );
    runtime.bootstrap_delivery = BootstrapDelivery::Delivered;
    atomic_write_json(&runtime_path, &runtime).unwrap();
    assert_eq!(
        host.creation_runtime_evidence(&binding, &planned),
        CreationRuntimeEvidence::Ready
    );

    std::fs::write(&launch_path, b"invalid-launch-evidence").unwrap();
    assert_eq!(
        host.creation_runtime_evidence(&binding, &planned),
        CreationRuntimeEvidence::Unknown
    );
}

#[tokio::test]
async fn creation_failure_resolution_accepts_a_proven_previous_boot() {
    let (_directory, mut host, _, binding, planned, mut runtime) =
        pending_creation_fixture(true).await;
    host.boot_id = CREATION_RUNTIME_CURRENT_BOOT.into();
    runtime.bootstrap_delivery = BootstrapDelivery::Failed;
    runtime.boot_id = CREATION_RUNTIME_PREVIOUS_BOOT.into();
    atomic_write_json(
        &host.paths.runtime_record(binding.result.terminal_id),
        &runtime,
    )
    .unwrap();
    assert_eq!(
        host.creation_runtime_evidence(&binding, &planned),
        CreationRuntimeEvidence::Failed(CreationFailureResolution::Resolved)
    );
}

#[test]
fn creation_process_identity_rejects_each_invalid_component() {
    const VALID_PROCESS_ID: u32 = 1;
    const VALID_START_TIME: u64 = 1;
    const OUT_OF_RANGE_PROCESS_ID: u32 = i32::MAX as u32 + 1;

    assert!(valid_creation_process_identity(
        VALID_PROCESS_ID,
        VALID_START_TIME
    ));
    assert!(!valid_creation_process_identity(0, VALID_START_TIME));
    assert!(!valid_creation_process_identity(
        OUT_OF_RANGE_PROCESS_ID,
        VALID_START_TIME
    ));
    assert!(!valid_creation_process_identity(VALID_PROCESS_ID, 0));
}

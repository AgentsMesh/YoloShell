use crate::host::tests::*;
use super::*;
const RESTORE_CONVERGENCE_BOOT_CURRENT: &str = "restore-convergence-current";
const RESTORE_CONVERGENCE_BOOT_PREVIOUS: &str = "restore-convergence-previous";
const RESTORE_CONVERGENCE_DEAD_PID: u32 = u32::MAX;
const RESTORE_CONVERGENCE_DEAD_START: u64 = 1;
const RESTORE_CONVERGENCE_SCHEMA_INCREMENT: u16 = 1;

fn restore_convergence_runtime(host: &Host, manifest: &TerminalManifest) -> RuntimeRecord {
    host_runtime_record(
        host,
        manifest.terminal_id,
        host.paths.worker_socket(manifest.terminal_id).unwrap(),
    )
}

fn restore_convergence_dead(record: &RuntimeRecord) -> RuntimeRecord {
    let mut dead = record.clone();
    let worker = reaped_test_process();
    let child = reaped_test_process();
    dead.worker_pid = worker.pid;
    dead.worker_start_time = worker.start_time;
    dead.child_pid = child.pid;
    dead.child_start_time = child.start_time;
    dead
}

#[test]
fn convergence_restore_decision_revalidates_runtime_and_pending_identity() {
    let directory = tempdir().unwrap();
    let mut host = Host::open(HostConfig::new(directory.path())).unwrap();
    host.boot_id = RESTORE_CONVERGENCE_BOOT_CURRENT.into();
    let manifest = fixture_manifest(directory.path());
    host.paths.ensure_session(manifest.terminal_id).unwrap();
    let runtime = restore_convergence_runtime(&host, &manifest);

    assert_eq!(
        host.automatic_restore_decision_for_runtime(&manifest, &runtime),
        AutomaticRestore::Ineligible
    );
    atomic_write_json(&host.paths.runtime_record(manifest.terminal_id), &runtime).unwrap();
    let mut expected = runtime.clone();
    expected.runtime_epoch = Uuid::new_v4();
    assert_eq!(
        host.automatic_restore_decision_for_runtime(&manifest, &expected),
        AutomaticRestore::Ineligible
    );
    assert_eq!(
        host.automatic_restore_decision_for_runtime(&manifest, &runtime),
        AutomaticRestore::BlockedByRuntime
    );

    let mut pending = runtime.clone();
    pending.terminal_id = Uuid::new_v4();
    pending.bootstrap_delivery = BootstrapDelivery::Pending;
    atomic_write_json(&host.paths.runtime_record(manifest.terminal_id), &pending).unwrap();
    assert!(!host.dead_pending_bootstrap(&manifest));
    pending.terminal_id = manifest.terminal_id;
    pending.bootstrap_delivery = BootstrapDelivery::NotRequired;
    atomic_write_json(&host.paths.runtime_record(manifest.terminal_id), &pending).unwrap();
    assert!(!host.dead_pending_bootstrap(&manifest));
    pending.bootstrap_delivery = BootstrapDelivery::Pending;
    atomic_write_json(&host.paths.runtime_record(manifest.terminal_id), &pending).unwrap();
    assert!(!host.dead_pending_bootstrap(&manifest));
    pending.child_pid = RESTORE_CONVERGENCE_DEAD_PID;
    pending.child_start_time = RESTORE_CONVERGENCE_DEAD_START;
    atomic_write_json(&host.paths.runtime_record(manifest.terminal_id), &pending).unwrap();
    assert!(host.dead_pending_bootstrap(&manifest));
}

#[tokio::test]
async fn convergence_automatic_restore_classifies_every_safety_fence() {
    let directory = tempdir().unwrap();
    let host = Host::open(HostConfig::new(directory.path())).unwrap();
    let manifest = fixture_manifest(directory.path());
    host.paths.ensure_session(manifest.terminal_id).unwrap();
    let runtime = restore_convergence_runtime(&host, &manifest);
    assert_eq!(
        host.classify_automatic_restore(&manifest, &runtime),
        AutomaticRestore::BlockedByRuntime
    );

    for state in [RecoveryState::NeedsRecreate, RecoveryState::Uncertain] {
        let mut blocked = manifest.clone();
        blocked.recovery_state = state;
        assert_eq!(
            host.classify_automatic_restore(&blocked, &runtime),
            AutomaticRestore::Ineligible
        );
    }
    let mut invalid = runtime.clone();
    invalid.schema_version += RESTORE_CONVERGENCE_SCHEMA_INCREMENT;
    assert_eq!(
        host.classify_automatic_restore(&manifest, &invalid),
        AutomaticRestore::Ineligible
    );
    invalid = runtime.clone();
    invalid.terminal_id = Uuid::new_v4();
    assert_eq!(
        host.classify_automatic_restore(&manifest, &invalid),
        AutomaticRestore::Ineligible
    );
    invalid = runtime.clone();
    invalid.bootstrap_delivery = BootstrapDelivery::Pending;
    assert_eq!(
        host.classify_automatic_restore(&manifest, &invalid),
        AutomaticRestore::Ineligible
    );

    let exit = ExitRecord {
        runtime_epoch: runtime.runtime_epoch,
        code: None,
        signal: None,
        reason: ExitReason::Natural,
        observed_at_ms: now_ms(),
        last_output_sequence: u64::default(),
    };
    atomic_write_json(&host.paths.exit_record(manifest.terminal_id), &exit).unwrap();
    assert_eq!(
        host.classify_automatic_restore(&manifest, &runtime),
        AutomaticRestore::Ineligible
    );
    std::fs::remove_file(host.paths.exit_record(manifest.terminal_id)).unwrap();
    let tombstone = crate::model::TerminationTombstone::user_request(
        manifest.terminal_id,
        runtime.runtime_epoch,
        false,
    );
    atomic_write_json(
        &host.paths.terminate_tombstone(manifest.terminal_id),
        &tombstone,
    )
    .unwrap();
    assert_eq!(
        host.classify_automatic_restore(&manifest, &runtime),
        AutomaticRestore::Ineligible
    );
    std::fs::remove_file(host.paths.terminate_tombstone(manifest.terminal_id)).unwrap();

    let mut previous = runtime.clone();
    previous.boot_id = RESTORE_CONVERGENCE_BOOT_PREVIOUS.into();
    assert_eq!(
        host.classify_automatic_restore(&manifest, &previous),
        AutomaticRestore::Eligible
    );
    let mut unknown_boot = runtime.clone();
    unknown_boot.boot_id = crate::process::unknown_boot_id_for_test();
    assert_eq!(
        host.classify_automatic_restore(&manifest, &unknown_boot),
        AutomaticRestore::BlockedByRuntime
    );
    for zero_worker in [true, false] {
        let mut incomplete = runtime.clone();
        if zero_worker {
            incomplete.worker_start_time = u64::default();
        } else {
            incomplete.child_start_time = u64::default();
        }
        assert_eq!(
            host.classify_automatic_restore(&manifest, &incomplete),
            AutomaticRestore::BlockedByRuntime
        );
        incomplete.boot_id = RESTORE_CONVERGENCE_BOOT_PREVIOUS.into();
        assert_eq!(
            host.classify_automatic_restore(&manifest, &incomplete),
            AutomaticRestore::Ineligible
        );
    }
    let dead = restore_convergence_dead(&runtime);
    {
        assert_eq!(
            host.classify_automatic_restore(&manifest, &dead),
            AutomaticRestore::BlockedByRuntime
        );
        host.secondary_custody.install_test_retired_lease(
            &host,
            &dead,
            secondary_custody::TestRetiredCustodyState::Released,
        );
    }
    assert_eq!(
        host.classify_automatic_restore(&manifest, &dead),
        AutomaticRestore::Eligible
    );
    let mut one_live = dead.clone();
    one_live.child_pid = runtime.child_pid;
    one_live.child_start_time = runtime.child_start_time;
    assert_eq!(
        host.classify_automatic_restore(&manifest, &one_live),
        AutomaticRestore::BlockedByRuntime
    );
    let mut live_worker = dead;
    live_worker.worker_pid = runtime.worker_pid;
    live_worker.worker_start_time = runtime.worker_start_time;
    assert_eq!(
        host.classify_automatic_restore(&manifest, &live_worker),
        AutomaticRestore::BlockedByRuntime
    );
}

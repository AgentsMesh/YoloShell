use crate::host::tests::*;
use super::*;

const DEAD_WORKER_SOCKET: &str = "dead-worker.sock";
const LIVE_WORKER_SOCKET: &str = "live-worker.sock";

#[test]
fn dead_unauthenticated_worker_is_projected_as_uncertain() {
    let directory = tempdir().unwrap();
    let host = Host::open(HostConfig::new(directory.path())).unwrap();
    let mut manifest = fixture_manifest(directory.path());
    manifest.recovery_state = RecoveryState::Reattached;
    let mut runtime = host_runtime_record(
        &host,
        manifest.terminal_id,
        directory.path().join(DEAD_WORKER_SOCKET),
    );
    let dead = reaped_test_process();
    runtime.worker_pid = dead.pid;
    runtime.worker_start_time = dead.start_time;
    let observation = RuntimeObservation::unauthenticated(unreachable_view(&host.paths, &runtime));

    host.project_runtime_recovery_state(&mut manifest, Some(&observation));

    assert_eq!(manifest.recovery_state, RecoveryState::Uncertain);
}

#[test]
fn live_unauthenticated_worker_keeps_its_durable_recovery_state() {
    let directory = tempdir().unwrap();
    let host = Host::open(HostConfig::new(directory.path())).unwrap();
    let mut manifest = fixture_manifest(directory.path());
    manifest.recovery_state = RecoveryState::Reattached;
    let runtime = host_runtime_record(
        &host,
        manifest.terminal_id,
        directory.path().join(LIVE_WORKER_SOCKET),
    );
    let observation = RuntimeObservation::unauthenticated(unreachable_view(&host.paths, &runtime));

    host.project_runtime_recovery_state(&mut manifest, Some(&observation));

    assert_eq!(manifest.recovery_state, RecoveryState::Reattached);
}

#[test]
fn dead_unauthenticated_worker_keeps_a_completed_recreation_state() {
    let directory = tempdir().unwrap();
    let host = Host::open(HostConfig::new(directory.path())).unwrap();
    let mut manifest = fixture_manifest(directory.path());
    manifest.recovery_state = RecoveryState::Recreated;
    let mut runtime = host_runtime_record(
        &host,
        manifest.terminal_id,
        directory.path().join(DEAD_WORKER_SOCKET),
    );
    let dead = reaped_test_process();
    runtime.worker_pid = dead.pid;
    runtime.worker_start_time = dead.start_time;
    let observation = RuntimeObservation::unauthenticated(unreachable_view(&host.paths, &runtime));

    host.project_runtime_recovery_state(&mut manifest, Some(&observation));

    assert_eq!(manifest.recovery_state, RecoveryState::Recreated);
}

#[test]
fn dead_unauthenticated_worker_keeps_stable_termination_state() {
    let directory = tempdir().unwrap();
    let host = Host::open(HostConfig::new(directory.path())).unwrap();
    let mut manifest = fixture_manifest(directory.path());
    host.paths.ensure_session(manifest.terminal_id).unwrap();
    manifest.recovery_state = RecoveryState::Reattached;
    let mut runtime = host_runtime_record(
        &host,
        manifest.terminal_id,
        directory.path().join(DEAD_WORKER_SOCKET),
    );
    let dead = reaped_test_process();
    runtime.worker_pid = dead.pid;
    runtime.worker_start_time = dead.start_time;
    host.write_namespace_json(&host.paths.runtime_record(manifest.terminal_id), &runtime)
        .unwrap();
    let tombstone = crate::model::TerminationTombstone::user_request(
        manifest.terminal_id,
        runtime.runtime_epoch,
        true,
    );
    host.write_namespace_json(
        &host.paths.terminate_tombstone(manifest.terminal_id),
        &tombstone,
    )
    .unwrap();
    let observation = RuntimeObservation::unauthenticated(unreachable_view(&host.paths, &runtime));

    host.project_runtime_recovery_state(&mut manifest, Some(&observation));

    assert_eq!(manifest.recovery_state, RecoveryState::Reattached);
}

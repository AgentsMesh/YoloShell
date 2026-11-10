use crate::host::tests::*;
use super::*;

const CURRENT_TEST_BOOT: &str = "current-test-boot";
const PREVIOUS_TEST_BOOT: &str = "previous-test-boot";

fn persist_runtime_for_resolution(host: &Host, record: &RuntimeRecord) {
    atomic_write_json(&host.paths.runtime_record(record.terminal_id), record).unwrap();
}

#[test]
fn runtime_spawn_resolution_requires_complete_matching_runtime_identity() {
    let directory = tempdir().unwrap();
    let mut host = Host::open(HostConfig::new(directory.path())).unwrap();
    host.boot_id = CURRENT_TEST_BOOT.into();
    let manifest = fixture_manifest(directory.path());
    host.paths.ensure_session(manifest.terminal_id).unwrap();
    assert!(!host.runtime_is_resolved_for_spawn(&manifest));

    let live = host_runtime_record(
        &host,
        manifest.terminal_id,
        host.paths.worker_socket(manifest.terminal_id).unwrap(),
    );
    persist_runtime_for_resolution(&host, &live);
    assert!(!host.runtime_is_resolved_for_spawn(&manifest));

    let mut invalid = live.clone();
    invalid.schema_version += 1;
    persist_runtime_for_resolution(&host, &invalid);
    assert!(!host.runtime_is_resolved_for_spawn(&manifest));

    invalid = live.clone();
    invalid.terminal_id = Uuid::new_v4();
    atomic_write_json(&host.paths.runtime_record(manifest.terminal_id), &invalid).unwrap();
    assert!(!host.runtime_is_resolved_for_spawn(&manifest));

    invalid = live.clone();
    invalid.worker_start_time = 0;
    persist_runtime_for_resolution(&host, &invalid);
    assert!(!host.runtime_is_resolved_for_spawn(&manifest));

    invalid = live.clone();
    invalid.child_start_time = 0;
    persist_runtime_for_resolution(&host, &invalid);
    assert!(!host.runtime_is_resolved_for_spawn(&manifest));
}

#[test]
fn runtime_spawn_resolution_handles_boot_and_both_process_identities() {
    let directory = tempdir().unwrap();
    let mut host = Host::open(HostConfig::new(directory.path())).unwrap();
    host.boot_id = CURRENT_TEST_BOOT.into();
    let manifest = fixture_manifest(directory.path());
    host.paths.ensure_session(manifest.terminal_id).unwrap();
    let mut record = host_runtime_record(
        &host,
        manifest.terminal_id,
        host.paths.worker_socket(manifest.terminal_id).unwrap(),
    );

    record.boot_id = PREVIOUS_TEST_BOOT.into();
    persist_runtime_for_resolution(&host, &record);
    assert!(host.runtime_is_resolved_for_spawn(&manifest));

    record.boot_id = CURRENT_TEST_BOOT.into();
    record.worker_pid = u32::MAX;
    record.worker_start_time = MISSING_PROCESS_START_TIME;
    persist_runtime_for_resolution(&host, &record);
    assert!(!host.runtime_is_resolved_for_spawn(&manifest));

    record.child_pid = u32::MAX;
    record.child_start_time = MISSING_PROCESS_START_TIME;
    persist_runtime_for_resolution(&host, &record);
    assert!(host.runtime_is_resolved_for_spawn(&manifest));
}

#[test]
fn pending_bootstrap_becomes_uncertain_only_after_an_identity_dies() {
    let directory = tempdir().unwrap();
    let host = Host::open(HostConfig::new(directory.path())).unwrap();
    let manifest = fixture_manifest(directory.path());
    host.paths.ensure_session(manifest.terminal_id).unwrap();
    let mut record = host_runtime_record(
        &host,
        manifest.terminal_id,
        host.paths.worker_socket(manifest.terminal_id).unwrap(),
    );
    record.bootstrap_delivery = BootstrapDelivery::Pending;
    persist_runtime_for_resolution(&host, &record);
    assert!(!host.dead_pending_bootstrap(&manifest));

    record.child_pid = u32::MAX;
    record.child_start_time = MISSING_PROCESS_START_TIME;
    persist_runtime_for_resolution(&host, &record);
    assert!(host.dead_pending_bootstrap(&manifest));
}

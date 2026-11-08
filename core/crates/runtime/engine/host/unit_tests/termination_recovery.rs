use crate::host::tests::*;
use super::*;

const CORRUPT_TOMBSTONE: &[u8] = b"not-json";
const UNSUPPORTED_SCHEMA_VERSION_OFFSET: u16 = 1;

pub(super) fn dead_runtime(host: &Host, manifest: &TerminalManifest) -> RuntimeRecord {
    let mut record = host_runtime_record(
        host,
        manifest.terminal_id,
        host.paths.worker_socket(manifest.terminal_id).unwrap(),
    );
    install_reaped_runtime_processes(&mut record);
    record
}

fn write_tombstone(host: &Host, tombstone: &crate::model::TerminationTombstone) {
    atomic_write_json(
        &host.paths.terminate_tombstone(tombstone.terminal_id),
        tombstone,
    )
    .unwrap();
}

#[test]
fn automatic_restore_requires_a_missing_termination_tombstone() {
    let directory = tempdir().unwrap();
    let host = Host::open(HostConfig::new(directory.path())).unwrap();
    let manifest = fixture_manifest(directory.path());
    let record = dead_runtime(&host, &manifest);
    persist_host_session(&host, &manifest, &record);
    host.secondary_custody
        .install_test_retirement_evidence(&host, &record);
    assert_eq!(
        host.automatic_restore_decision(&manifest),
        AutomaticRestore::Eligible
    );

    let matching = crate::model::TerminationTombstone::user_request(
        manifest.terminal_id,
        record.runtime_epoch,
        true,
    );
    write_tombstone(&host, &matching);
    assert_eq!(
        host.termination_tombstone_state(&record),
        TerminationTombstoneState::Matching
    );
    assert_eq!(
        host.automatic_restore_decision(&manifest),
        AutomaticRestore::Ineligible
    );
}

#[test]
fn corrupt_and_mismatched_termination_tombstones_fail_closed() {
    let directory = tempdir().unwrap();
    let host = Host::open(HostConfig::new(directory.path())).unwrap();
    let manifest = fixture_manifest(directory.path());
    let record = dead_runtime(&host, &manifest);
    persist_host_session(&host, &manifest, &record);
    let path = host.paths.terminate_tombstone(manifest.terminal_id);
    atomic_write_json(&path, &serde_json::json!({})).unwrap();
    std::fs::write(&path, CORRUPT_TOMBSTONE).unwrap();
    assert_invalid_tombstone(&host, &manifest, &record);

    let wrong_session = crate::model::TerminationTombstone::user_request(
        Uuid::new_v4(),
        record.runtime_epoch,
        false,
    );
    atomic_write_json(&path, &wrong_session).unwrap();
    assert_invalid_tombstone(&host, &manifest, &record);

    let wrong_runtime = crate::model::TerminationTombstone::user_request(
        manifest.terminal_id,
        Uuid::new_v4(),
        false,
    );
    write_tombstone(&host, &wrong_runtime);
    assert_invalid_tombstone(&host, &manifest, &record);
}

fn assert_invalid_tombstone(host: &Host, manifest: &TerminalManifest, record: &RuntimeRecord) {
    assert_eq!(
        host.termination_tombstone_state(record),
        TerminationTombstoneState::Invalid
    );
    assert_eq!(
        host.automatic_restore_decision(manifest),
        AutomaticRestore::Ineligible
    );
}

#[tokio::test]
async fn host_termination_of_a_dead_runtime_persists_the_same_epoch() {
    let directory = tempdir().unwrap();
    let host = Host::open(HostConfig::new(directory.path())).unwrap();
    let manifest = fixture_manifest(directory.path());
    let record = dead_runtime(&host, &manifest);
    persist_host_session(&host, &manifest, &record);
    persist_archive_exit(&host, &record, record.runtime_epoch);

    host.terminate_bound_session(manifest.terminal_id, true)
        .await
        .unwrap();

    let tombstone: crate::model::TerminationTombstone =
        read_json(&host.paths.terminate_tombstone(manifest.terminal_id)).unwrap();
    assert_eq!(
        tombstone.validate_for(manifest.terminal_id, record.runtime_epoch),
        Ok(())
    );
    assert!(tombstone.force);
    assert_eq!(
        host.automatic_restore_decision(&manifest),
        AutomaticRestore::Ineligible
    );
}

#[tokio::test]
async fn host_termination_never_acks_an_untrusted_or_partly_live_runtime() {
    let directory = tempdir().unwrap();
    let host = Host::open(HostConfig::new(directory.path())).unwrap();
    let manifest = fixture_manifest(directory.path());
    let mut record = dead_runtime(&host, &manifest);
    record.child_pid = std::process::id();
    record.child_start_time = process::process_start_time(record.child_pid).unwrap();
    persist_host_session(&host, &manifest, &record);
    assert!(
        host.terminate_bound_session(manifest.terminal_id, false)
            .await
            .is_err()
    );
    assert!(!host.paths.terminate_tombstone(manifest.terminal_id).exists());

    record.schema_version += UNSUPPORTED_SCHEMA_VERSION_OFFSET;
    atomic_write_json(&host.paths.runtime_record(manifest.terminal_id), &record).unwrap();
    assert!(
        host.terminate_bound_session(manifest.terminal_id, false)
            .await
            .is_err()
    );
    assert!(!host.paths.terminate_tombstone(manifest.terminal_id).exists());
}

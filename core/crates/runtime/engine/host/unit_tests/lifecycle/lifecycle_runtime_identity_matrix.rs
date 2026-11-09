use crate::host::tests::*;
use super::*;

const CORRUPT_RUNTIME_IDENTITY_BYTES: &[u8] = b"not-runtime-json";
const CURRENT_RUNTIME_BOOT_ID: &str = "known-current-runtime-boot";
const PREVIOUS_RUNTIME_BOOT_ID: &str = "known-previous-runtime-boot";
const RUNTIME_SCHEMA_OFFSET: u16 = 1;

#[derive(Clone, Copy, Debug)]
enum UnverifiableRuntime {
    Schema,
    Session,
    WorkerStart,
    ChildStart,
    LiveChild,
}

impl UnverifiableRuntime {
    fn apply(self, record: &mut RuntimeRecord) {
        match self {
            Self::Schema => record.schema_version += RUNTIME_SCHEMA_OFFSET,
            Self::Session => record.terminal_id = Uuid::new_v4(),
            Self::WorkerStart => record.worker_start_time = u64::default(),
            Self::ChildStart => record.child_start_time = u64::default(),
            Self::LiveChild => {
                record.child_pid = std::process::id();
                record.child_start_time = process::process_start_time(record.child_pid).unwrap();
            }
        }
    }
}

#[test]
fn dead_runtime_termination_requires_complete_trusted_identity_evidence() {
    let directory = tempdir().unwrap();
    let mut host = Host::open(HostConfig::new(directory.path())).unwrap();
    host.boot_id = CURRENT_RUNTIME_BOOT_ID.into();
    let manifest = fixture_manifest(directory.path());
    let path = host.paths.runtime_record(manifest.terminal_id);
    host.paths.ensure_session(manifest.terminal_id).unwrap();

    assert!(
        !host
            .persist_dead_runtime_termination(manifest.terminal_id, false)
            .unwrap()
    );
    write_private_bytes(&path, CORRUPT_RUNTIME_IDENTITY_BYTES);
    assert!(
        !host
            .persist_dead_runtime_termination(manifest.terminal_id, false)
            .unwrap()
    );

    for mutation in [
        UnverifiableRuntime::Schema,
        UnverifiableRuntime::Session,
        UnverifiableRuntime::WorkerStart,
        UnverifiableRuntime::ChildStart,
        UnverifiableRuntime::LiveChild,
    ] {
        let mut record = dead_runtime(&host, &manifest);
        mutation.apply(&mut record);
        atomic_write_json(&path, &record).unwrap();
        assert!(
            !host
                .persist_dead_runtime_termination(manifest.terminal_id, false)
                .unwrap(),
            "unexpected termination acknowledgement for {mutation:?}"
        );
    }

    let mut previous_boot = host_runtime_record(
        &host,
        manifest.terminal_id,
        host.paths.worker_socket(manifest.terminal_id).unwrap(),
    );
    previous_boot.boot_id = PREVIOUS_RUNTIME_BOOT_ID.into();
    atomic_write_json(&path, &previous_boot).unwrap();
    assert!(
        host.persist_dead_runtime_termination(manifest.terminal_id, true)
            .unwrap()
    );
    let tombstone: crate::model::TerminationTombstone =
        read_json(&host.paths.terminate_tombstone(manifest.terminal_id)).unwrap();
    assert_eq!(tombstone.runtime_epoch, previous_boot.runtime_epoch);
    assert!(tombstone.force);
}

#[test]
fn runtime_liveness_fails_closed_for_missing_or_partial_evidence() {
    let directory = tempdir().unwrap();
    let host = Host::open(HostConfig::new(directory.path())).unwrap();
    let manifest = fixture_manifest(directory.path());
    let path = host.paths.runtime_record(manifest.terminal_id);

    assert!(!host.runtime_record_may_be_live(manifest.terminal_id));
    host.paths.ensure_session(manifest.terminal_id).unwrap();
    write_private_bytes(&path, CORRUPT_RUNTIME_IDENTITY_BYTES);
    assert!(host.runtime_record_may_be_live(manifest.terminal_id));

    let dead = dead_runtime(&host, &manifest);
    atomic_write_json(&path, &dead).unwrap();
    assert!(!host.runtime_record_may_be_live(manifest.terminal_id));
    for mutation in [
        UnverifiableRuntime::Schema,
        UnverifiableRuntime::Session,
        UnverifiableRuntime::WorkerStart,
        UnverifiableRuntime::ChildStart,
        UnverifiableRuntime::LiveChild,
    ] {
        let mut record = dead.clone();
        mutation.apply(&mut record);
        atomic_write_json(&path, &record).unwrap();
        assert!(
            host.runtime_record_may_be_live(manifest.terminal_id),
            "liveness failed open for {mutation:?}"
        );
    }
}

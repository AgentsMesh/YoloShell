use crate::host::tests::*;
use super::*;

const ARCHIVE_EVIDENCE_SOCKET_NAME: &str = "archive-evidence.sock";
const ARCHIVE_EVIDENCE_ERROR: &str = "archive runtime evidence unavailable";
const CORRUPT_ARCHIVE_RUNTIME_BYTES: &[u8] = b"not-archive-runtime-json";
const ARCHIVE_IDENTITY_TEXT_SUFFIX: &str = "-replacement";
const ARCHIVE_SCHEMA_OFFSET: u16 = 1;
const ARCHIVE_PID_OFFSET: u32 = 1;
const ARCHIVE_START_OFFSET: u64 = 1;

#[derive(Clone, Copy, Debug)]
enum ArchiveIdentityMutation {
    Schema,
    Session,
    RuntimeEpoch,
    WorkerId,
    WorkerPid,
    WorkerStart,
    ChildPid,
    ChildStart,
    Boot,
    Socket,
    AuthToken,
}

impl ArchiveIdentityMutation {
    fn apply(self, record: &mut RuntimeRecord) {
        match self {
            Self::Schema => record.schema_version += ARCHIVE_SCHEMA_OFFSET,
            Self::Session => record.terminal_id = Uuid::new_v4(),
            Self::RuntimeEpoch => record.runtime_epoch = Uuid::new_v4(),
            Self::WorkerId => record.worker_id = Uuid::new_v4(),
            Self::WorkerPid => {
                record.worker_pid = record.worker_pid.saturating_sub(ARCHIVE_PID_OFFSET)
            }
            Self::WorkerStart => {
                record.worker_start_time = record
                    .worker_start_time
                    .saturating_add(ARCHIVE_START_OFFSET)
            }
            Self::ChildPid => {
                record.child_pid = record.child_pid.saturating_sub(ARCHIVE_PID_OFFSET)
            }
            Self::ChildStart => {
                record.child_start_time =
                    record.child_start_time.saturating_add(ARCHIVE_START_OFFSET)
            }
            Self::Boot => record.boot_id.push_str(ARCHIVE_IDENTITY_TEXT_SUFFIX),
            Self::Socket => record.socket_path.push(ARCHIVE_IDENTITY_TEXT_SUFFIX),
            Self::AuthToken => record.auth_token.push_str(ARCHIVE_IDENTITY_TEXT_SUFFIX),
        }
    }

    fn invalidate_verifiable(self, record: &mut RuntimeRecord) {
        match self {
            Self::WorkerStart => record.worker_start_time = u64::default(),
            Self::ChildStart => record.child_start_time = u64::default(),
            _ => self.apply(record),
        }
    }
}

#[test]
fn archive_runtime_identity_rejects_every_identity_field_replacement() {
    let directory = tempdir().unwrap();
    let host = Host::open(HostConfig::new(directory.path())).unwrap();
    let selected = dead_archive_record(
        &host,
        Uuid::new_v4(),
        directory.path().join(ARCHIVE_EVIDENCE_SOCKET_NAME),
    );
    assert!(selected.has_same_process_identity(&selected));

    for mutation in [
        ArchiveIdentityMutation::Schema,
        ArchiveIdentityMutation::Session,
        ArchiveIdentityMutation::RuntimeEpoch,
        ArchiveIdentityMutation::WorkerId,
        ArchiveIdentityMutation::WorkerPid,
        ArchiveIdentityMutation::WorkerStart,
        ArchiveIdentityMutation::ChildPid,
        ArchiveIdentityMutation::ChildStart,
        ArchiveIdentityMutation::Boot,
        ArchiveIdentityMutation::Socket,
        ArchiveIdentityMutation::AuthToken,
    ] {
        let mut replacement = selected.clone();
        mutation.apply(&mut replacement);
        assert!(
            !replacement.has_same_process_identity(&selected),
            "archive accepted replaced {mutation:?}"
        );
    }
}

#[test]
fn persisted_archive_runtime_distinguishes_missing_corrupt_and_invalid_evidence() {
    let directory = tempdir().unwrap();
    let host = Host::open(HostConfig::new(directory.path())).unwrap();
    let terminal_id = Uuid::new_v4();
    let path = host.paths.runtime_record(terminal_id);
    let live_error = || ErrorBody::conflict(ARCHIVE_EVIDENCE_ERROR);

    assert!(
        host.persisted_archive_runtime(terminal_id, live_error())
            .unwrap()
            .is_none()
    );
    host.paths.ensure_session(terminal_id).unwrap();
    write_private_bytes(&path, CORRUPT_ARCHIVE_RUNTIME_BYTES);
    let corrupt = host
        .persisted_archive_runtime(terminal_id, live_error())
        .unwrap_err();
    assert_eq!(corrupt.message, ARCHIVE_EVIDENCE_ERROR);

    let valid = dead_archive_record(
        &host,
        terminal_id,
        directory.path().join(ARCHIVE_EVIDENCE_SOCKET_NAME),
    );
    for mutation in [
        ArchiveIdentityMutation::Schema,
        ArchiveIdentityMutation::Session,
        ArchiveIdentityMutation::WorkerStart,
        ArchiveIdentityMutation::ChildStart,
    ] {
        let mut invalid = valid.clone();
        mutation.invalidate_verifiable(&mut invalid);
        atomic_write_json(&path, &invalid).unwrap();
        assert!(
            host.persisted_archive_runtime(terminal_id, live_error())
                .is_err(),
            "archive trusted invalid {mutation:?}"
        );
    }
    atomic_write_json(&path, &valid).unwrap();
    let selected = host
        .persisted_archive_runtime(terminal_id, live_error())
        .unwrap()
        .unwrap();
    assert_eq!(selected.runtime_epoch, valid.runtime_epoch);
    assert!(runtime_loss::runtime_record_identity_is_verifiable(&selected, terminal_id));
}

#[tokio::test]
async fn archive_settles_when_shutdown_transport_drops_after_durable_exit() {
    let directory = tempdir().unwrap();
    let host = Host::open(HostConfig::new(directory.path())).unwrap();
    let socket = directory.path().join(ARCHIVE_EVIDENCE_SOCKET_NAME);
    let mut worker = ArchiveWorkerProcessFixture::spawn(
        socket.clone(),
        ArchiveWorkerFixtureMode::ExitedThenShutdownDropBoth,
    );
    let mut record = host_runtime_record(&host, Uuid::new_v4(), socket);
    worker.configure(&mut record);
    persist_archive_runtime(&host, &record);
    persist_archive_exit(&host, &record, record.runtime_epoch);

    host.settle_archive_runtime(&record, PROCESS_EXIT_TIMEOUT)
        .await
        .unwrap();
    worker.reap_completed_worker();
}

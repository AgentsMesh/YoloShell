use super::runtime_record_authority::BoundRuntimeRecord;
use super::runtime_record_test_support::{record_for_identity, runtime_view, session_view};
use super::*;

const INVALID_SESSION_ENTRY_BYTES: &[u8] = b"not-a-session-directory";
const AUTH_TOKEN_MUTATION_SUFFIX: &str = "-changed";
const BOOT_ID_MUTATION_SUFFIX: &str = "-changed";
const IDENTITY_MUTATION_INCREMENT: u64 = 1;
const SOCKET_PATH_MUTATION_COMPONENT: &str = "changed";
const TEST_PROCESS_ARGUMENT: &str = "60";
const TEST_PROCESS_EXECUTABLE: &str = "/bin/sleep";

#[test]
fn forged_disk_process_identity_never_receives_cleanup_signals() {
    let directory = tempfile::tempdir().unwrap();
    let paths = Paths::new(directory.path());
    let mut victim = ChildGuard::new(
        Command::new(TEST_PROCESS_EXECUTABLE)
            .arg(TEST_PROCESS_ARGUMENT)
            .spawn()
            .unwrap(),
    );
    let identity = ProcessIdentity::capture(victim.id()).unwrap();
    let record = record_for_identity(&paths, Uuid::new_v4(), identity);
    yolo_runtime::paths::atomic_write_json(&paths.runtime_record(record.terminal_id), &record)
        .unwrap();

    let (mut cleanup, capture) = RuntimeCleanup::capture(&paths);
    assert!(capture.is_err());
    cleanup.request();
    cleanup.force();
    assert!(cleanup.finish().is_err());
    assert!(identity.matches());

    victim.kill().unwrap();
    victim.wait().unwrap();
}

#[test]
fn capture_and_rescan_report_authority_errors_without_forgetting_them() {
    let first = tempfile::tempdir().unwrap();
    let first_paths = Paths::new(first.path());
    write_invalid_session_entry(&first_paths);
    let (mut failed_capture, capture) = RuntimeCleanup::capture(&first_paths);
    assert!(capture.is_err());
    assert!(failed_capture.finish().is_err());

    let second = tempfile::tempdir().unwrap();
    let second_paths = Paths::new(second.path());
    let (mut cleanup, capture) = RuntimeCleanup::capture(&second_paths);
    capture.unwrap();
    write_invalid_session_entry(&second_paths);
    assert!(cleanup.rescan(&second_paths).is_err());
    assert!(cleanup.finish().is_err());
}

#[test]
fn failed_view_binding_is_reported_immediately_and_retained_for_finish() {
    let directory = tempfile::tempdir().unwrap();
    let paths = Paths::new(directory.path());
    let identity = ProcessIdentity::capture(std::process::id()).unwrap();
    let record = record_for_identity(&paths, Uuid::new_v4(), identity);
    let (mut cleanup, capture) = RuntimeCleanup::capture(&paths);
    capture.unwrap();
    yolo_runtime::paths::atomic_write_json(&paths.runtime_record(record.terminal_id), &record)
        .unwrap();
    let mut mismatched = session_view(&record);
    mismatched.runtime.as_mut().unwrap().worker_id = Uuid::new_v4();

    assert!(cleanup.bind_view(&paths, &mismatched).is_err());
    assert!(cleanup.finish().is_err());
}

#[test]
fn trusted_runtime_view_rejects_every_destructive_identity_mismatch() {
    let directory = tempfile::tempdir().unwrap();
    let paths = Paths::new(directory.path());
    let identity = ProcessIdentity::capture(std::process::id()).unwrap();
    let record = record_for_identity(&paths, Uuid::new_v4(), identity);
    let trusted = runtime_view(&record);
    let mutations = [
        AuthorityMutation::Session,
        AuthorityMutation::Epoch,
        AuthorityMutation::Worker,
        AuthorityMutation::WorkerPid,
        AuthorityMutation::WorkerStart,
        AuthorityMutation::ChildPid,
        AuthorityMutation::ChildStart,
        AuthorityMutation::Boot,
        AuthorityMutation::Socket,
    ];
    for mutation in mutations {
        let (candidate, terminal_id) = mutation.apply(record.clone());
        assert!(BoundRuntimeRecord::bind_runtime(&paths, candidate, terminal_id, &trusted).is_err());
    }
}

#[test]
fn bound_runtime_authority_rejects_auth_token_replacement() {
    let directory = tempfile::tempdir().unwrap();
    let paths = Paths::new(directory.path());
    let identity = ProcessIdentity::capture(std::process::id()).unwrap();
    let record = record_for_identity(&paths, Uuid::new_v4(), identity);
    let trusted = runtime_view(&record);
    yolo_runtime::paths::atomic_write_json(&paths.runtime_record(record.terminal_id), &record)
        .unwrap();
    let bound =
        BoundRuntimeRecord::bind_runtime(&paths, record.clone(), record.terminal_id, &trusted)
            .unwrap();
    let mut replacement = record;
    replacement.auth_token.push_str(AUTH_TOKEN_MUTATION_SUFFIX);

    assert!(bound.refresh(&replacement).is_err());
}

#[derive(Clone, Copy)]
enum AuthorityMutation {
    Session,
    Epoch,
    Worker,
    WorkerPid,
    WorkerStart,
    ChildPid,
    ChildStart,
    Boot,
    Socket,
}

impl AuthorityMutation {
    fn apply(self, mut record: RuntimeRecord) -> (RuntimeRecord, Uuid) {
        let mut terminal_id = record.terminal_id;
        match self {
            Self::Session => terminal_id = Uuid::new_v4(),
            Self::Epoch => record.runtime_epoch = Uuid::new_v4(),
            Self::Worker => record.worker_id = Uuid::new_v4(),
            Self::WorkerPid => {
                record.worker_pid = record
                    .worker_pid
                    .saturating_add(u32::try_from(IDENTITY_MUTATION_INCREMENT).unwrap())
            }
            Self::WorkerStart => {
                record.worker_start_time = record
                    .worker_start_time
                    .saturating_add(IDENTITY_MUTATION_INCREMENT)
            }
            Self::ChildPid => {
                record.child_pid = record
                    .child_pid
                    .saturating_add(u32::try_from(IDENTITY_MUTATION_INCREMENT).unwrap())
            }
            Self::ChildStart => {
                record.child_start_time = record
                    .child_start_time
                    .saturating_add(IDENTITY_MUTATION_INCREMENT)
            }
            Self::Boot => record.boot_id.push_str(BOOT_ID_MUTATION_SUFFIX),
            Self::Socket => record.socket_path.push(SOCKET_PATH_MUTATION_COMPONENT),
        }
        (record, terminal_id)
    }
}

fn write_invalid_session_entry(paths: &Paths) {
    std::fs::create_dir_all(paths.sessions_dir()).unwrap();
    std::fs::write(
        paths.sessions_dir().join(Uuid::new_v4().to_string()),
        INVALID_SESSION_ENTRY_BYTES,
    )
    .unwrap();
}

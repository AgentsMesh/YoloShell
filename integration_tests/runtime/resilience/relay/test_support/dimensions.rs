use super::*;
use std::path::PathBuf;
use uuid::Uuid;
use yolo_runtime::paths::atomic_write_json;

const AUTH_TOKEN: &str = "relay-dimensions-fixture";
const COMMAND: &str = "/bin/sh";
const INITIAL_GENERATION: u64 = 1;
const INITIAL_COLS: u16 = 90;
const INITIAL_ROWS: u16 = 25;
const RESIZED_COLS: u16 = 101;
const RESIZED_ROWS: u16 = 36;

pub(super) struct Fixture {
    _directory: tempfile::TempDir,
    pub path: PathBuf,
    pub subject: RuntimeDimensions,
    pub resized: RuntimeRecord,
    pub size: TerminalSize,
}

impl Fixture {
    pub fn new() -> Self {
        let directory = tempfile::tempdir().unwrap();
        let paths = Paths::new(directory.path());
        let terminal_id = Uuid::new_v4();
        paths.ensure_session(terminal_id).unwrap();
        let identity =
            yolo_process_test_support::ProcessIdentity::capture(std::process::id()).unwrap();
        let record = RuntimeRecord {
            schema_version: yolo_runtime::constants::RUNTIME_RECORD_SCHEMA_VERSION,
            terminal_id,
            runtime_epoch: Uuid::new_v4(),
            worker_id: Uuid::new_v4(),
            worker_pid: identity.pid,
            worker_start_time: identity.start_time,
            child_pid: identity.pid,
            child_start_time: identity.start_time,
            boot_id: yolo_process_test_support::boot_id(),
            socket_path: paths.worker_socket(terminal_id).unwrap(),
            auth_token: AUTH_TOKEN.into(),
            controller_generation: INITIAL_GENERATION,
            started_at_ms: i64::default(),
            command: COMMAND.into(),
            args: Vec::new(),
            cwd: directory.path().to_path_buf(),
            cols: INITIAL_COLS,
            rows: INITIAL_ROWS,
            bootstrap_delivery: yolo_runtime::model::BootstrapDelivery::NotRequired,
        };
        let path = paths.runtime_record(terminal_id);
        atomic_write_json(&path, &record).unwrap();
        let mut resized = record.clone();
        resized.cols = RESIZED_COLS;
        resized.rows = RESIZED_ROWS;
        Self {
            _directory: directory,
            subject: RuntimeDimensions {
                entry: PrivateFileEntry::open(&path).unwrap(),
                expected: record,
            },
            path,
            resized,
            size: TerminalSize::new(RESIZED_COLS, RESIZED_ROWS).unwrap(),
        }
    }

    pub fn conflicted_read(
        entry: &PrivateFileEntry,
        next: &RuntimeRecord,
    ) -> Result<RuntimeRecord> {
        read_entry_guarded(entry.reopen()?, || {
            std::fs::write(entry.path(), serde_json::to_vec(next)?)?;
            Ok(())
        })
    }
}

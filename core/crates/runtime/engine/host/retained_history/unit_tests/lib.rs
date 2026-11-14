use base64::Engine;
use std::os::unix::fs::PermissionsExt;
use tempfile::TempDir;
use uuid::Uuid;
use yolo_host_retained_history::read;
use yolo_journal::{Journal, write_snapshot};
use yolo_paths::namespace_lease::NamespaceLease;
use yolo_paths::Paths;
use yolo_runtime_protocol::RetainedHistoryRequest;
use yolo_terminal_model::{
    BootstrapDelivery, RecoveryState, RuntimeRecord, TerminalManifest, TerminalSnapshot,
};

fn private(path: &std::path::Path) {
    std::fs::create_dir_all(path).unwrap();
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o700)).unwrap();
}

fn fixture() -> (TempDir, NamespaceLease, Paths, Uuid, Uuid) {
    let temporary = TempDir::new().unwrap();
    private(temporary.path());
    let paths = Paths::new(temporary.path());
    let terminal_id = Uuid::new_v4();
    let runtime_epoch = Uuid::new_v4();
    private(&paths.sessions_dir());
    private(&paths.session_dir(terminal_id));
    private(&paths.terminal_dir(terminal_id));
    let namespace = NamespaceLease::capture(&[("test", temporary.path())]).unwrap();
    let manifest = TerminalManifest {
        schema_version: 1,
        terminal_id,
        name: "archived".into(),
        command: "bash".into(),
        args: Vec::new(),
        cwd: "/tmp".into(),
        cols: 80,
        rows: 24,
        created_at_ms: 1,
        updated_at_ms: 2,
        archived: true,
        recovery_state: RecoveryState::Created,
    };
    let runtime = RuntimeRecord {
        schema_version: 1,
        terminal_id,
        runtime_epoch,
        worker_id: Uuid::new_v4(),
        worker_pid: 1,
        worker_start_time: 1,
        child_pid: 2,
        child_start_time: 2,
        boot_id: "boot".into(),
        socket_path: "/tmp/worker.sock".into(),
        auth_token: "token".into(),
        controller_generation: 1,
        started_at_ms: 1,
        command: "bash".into(),
        args: Vec::new(),
        cwd: "/tmp".into(),
        cols: 80,
        rows: 24,
        bootstrap_delivery: BootstrapDelivery::NotRequired,
    };
    namespace.write_json(&paths.manifest(terminal_id), &manifest).unwrap();
    namespace
        .write_json(&paths.runtime_record(terminal_id), &runtime)
        .unwrap();
    (temporary, namespace, paths, terminal_id, runtime_epoch)
}

#[test]
fn archived_history_is_the_retained_raw_stream() {
    let (_temporary, namespace, paths, terminal_id, runtime_epoch) = fixture();
    let mut journal = Journal::open(paths.journal(terminal_id), runtime_epoch).unwrap();
    journal.append(b"hello ").unwrap();
    journal.append(b"world").unwrap();
    let result = read(
        &namespace,
        &paths,
        RetainedHistoryRequest {
            terminal_id,
            runtime_epoch,
            max_bytes: 1024,
        },
    )
    .unwrap();
    let data = base64::engine::general_purpose::STANDARD
        .decode(result.data_base64)
        .unwrap();
    assert_eq!(data, b"hello world");
    assert_eq!(result.from_sequence, 1);
    assert_eq!(result.next_sequence, 3);
    assert!(!result.truncated);
}

#[test]
fn live_history_is_rejected_and_the_budget_is_bounded() {
    let (_temporary, namespace, paths, terminal_id, runtime_epoch) = fixture();
    let mut manifest: TerminalManifest = namespace.read_json(&paths.manifest(terminal_id)).unwrap();
    manifest.archived = false;
    namespace.write_json(&paths.manifest(terminal_id), &manifest).unwrap();
    let error = read(
        &namespace,
        &paths,
        RetainedHistoryRequest {
            terminal_id,
            runtime_epoch,
            max_bytes: 1024,
        },
    )
    .unwrap_err();
    assert_eq!(error.code, yolo_wire::error_code::CONFLICT);
    let manifest_error = read(
        &namespace,
        &paths,
        RetainedHistoryRequest {
            terminal_id,
            runtime_epoch,
            max_bytes: 0,
        },
    )
    .unwrap_err();
    assert_eq!(manifest_error.code, yolo_wire::error_code::INVALID_REQUEST);
}

#[test]
fn missing_journal_is_not_found() {
    let (_temporary, namespace, paths, terminal_id, runtime_epoch) = fixture();
    let error = read(
        &namespace,
        &paths,
        RetainedHistoryRequest {
            terminal_id,
            runtime_epoch,
            max_bytes: 1024,
        },
    )
    .unwrap_err();
    assert_eq!(error.code, yolo_wire::error_code::NOT_FOUND);
}

#[test]
fn a_compacted_journal_uses_the_retained_snapshot() {
    let (_temporary, namespace, paths, terminal_id, runtime_epoch) = fixture();
    let mut journal = Journal::open(paths.journal(terminal_id), runtime_epoch).unwrap();
    journal.append(b"one").unwrap();
    journal.append(b"two").unwrap();
    journal.compact_through(2).unwrap();
    write_snapshot(
        &paths.snapshot(terminal_id),
        &TerminalSnapshot {
            runtime_epoch,
            barrier_sequence: 2,
            cols: 80,
            rows: 24,
            data: b"screen".to_vec(),
        },
    )
    .unwrap();
    let result = read(
        &namespace,
        &paths,
        RetainedHistoryRequest {
            terminal_id,
            runtime_epoch,
            max_bytes: 1024,
        },
    )
    .unwrap();
    let data = base64::engine::general_purpose::STANDARD
        .decode(result.data_base64)
        .unwrap();
    assert_eq!(data, b"screen");
    assert_eq!(result.from_sequence, 2);
    assert!(result.truncated);
}

#[test]
fn the_byte_budget_truncates_the_oldest_retained_output() {
    let (_temporary, namespace, paths, terminal_id, runtime_epoch) = fixture();
    let mut journal = Journal::open(paths.journal(terminal_id), runtime_epoch).unwrap();
    journal.append(b"abc").unwrap();
    journal.append(b"def").unwrap();
    let result = read(
        &namespace,
        &paths,
        RetainedHistoryRequest {
            terminal_id,
            runtime_epoch,
            max_bytes: 4,
        },
    )
    .unwrap();
    let data = base64::engine::general_purpose::STANDARD
        .decode(result.data_base64)
        .unwrap();
    assert_eq!(data, b"abc");
    assert!(result.truncated);
}

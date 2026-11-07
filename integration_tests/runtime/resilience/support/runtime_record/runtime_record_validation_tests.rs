use super::cleanup::collect_runtime_records;
use super::runtime_record_test_support::bind_record;
use super::runtime_record_validation::{readable_runtime_records, validate_runtime_record};
use super::*;

const TEST_COMMAND: &str = "/bin/cat";
const TEST_AUTH_TOKEN: &str = "runtime-validation-token";
const TAMPERED_AUTH_TOKEN: &str = "tampered-runtime-token";
const NON_SESSION_ENTRY: &str = "not-a-session";
const TEST_COLUMNS: u16 = 80;
const TEST_ROWS: u16 = 24;
const TEST_GENERATION: u64 = 1;
const ADOPTED_TEST_GENERATION: u64 = TEST_GENERATION + 1;
const TEST_STARTED_AT_MS: i64 = 1;
const INVALID_SCHEMA_VERSION: u16 = u16::MAX;
const FOREIGN_BOOT_ID: &str = "foreign-boot";
const CORRUPT_RUNTIME_BYTES: &[u8] = b"not-a-runtime-record";

#[test]
fn copied_runtime_record_cannot_cross_home_socket_namespaces() {
    let source = tempfile::tempdir().unwrap();
    let target = tempfile::tempdir().unwrap();
    let source_paths = Paths::new(source.path());
    let target_paths = Paths::new(target.path());
    let terminal_id = Uuid::new_v4();
    target_paths.ensure_session(terminal_id).unwrap();
    let record = fixture_record(&source_paths, terminal_id);
    yolo_runtime::paths::atomic_write_json(&target_paths.runtime_record(terminal_id), &record)
        .unwrap();

    let scan = readable_runtime_records(&target_paths);

    assert!(scan.records.is_empty());
    assert!(!scan.failures.is_empty());
}

#[test]
fn moved_runtime_record_cannot_claim_another_session_directory() {
    let directory = tempfile::tempdir().unwrap();
    let paths = Paths::new(directory.path());
    let record_session = Uuid::new_v4();
    let directory_session = Uuid::new_v4();
    paths.ensure_session(directory_session).unwrap();
    let record = fixture_record(&paths, record_session);
    yolo_runtime::paths::atomic_write_json(&paths.runtime_record(directory_session), &record)
        .unwrap();

    let scan = readable_runtime_records(&paths);

    assert!(scan.records.is_empty());
    assert!(!scan.failures.is_empty());
}

#[test]
fn corrupt_runtime_does_not_discard_other_valid_records() {
    let directory = tempfile::tempdir().unwrap();
    let paths = Paths::new(directory.path());
    let valid_session = Uuid::new_v4();
    let corrupt_session = Uuid::new_v4();
    paths.ensure_session(valid_session).unwrap();
    paths.ensure_session(corrupt_session).unwrap();
    let valid = fixture_record(&paths, valid_session);
    yolo_runtime::paths::atomic_write_json(&paths.runtime_record(valid_session), &valid).unwrap();
    std::fs::write(paths.runtime_record(corrupt_session), CORRUPT_RUNTIME_BYTES).unwrap();

    let scan = readable_runtime_records(&paths);

    assert_eq!(scan.records, [valid]);
    assert!(!scan.failures.is_empty());
}

#[test]
fn bound_runtime_survives_loss_of_its_durable_home() {
    let directory = tempfile::tempdir().unwrap();
    let paths = Paths::new(directory.path());
    let terminal_id = Uuid::new_v4();
    paths.ensure_session(terminal_id).unwrap();
    let record = fixture_record(&paths, terminal_id);
    let bound = bind_record(&paths, record.clone());
    std::fs::remove_dir_all(directory.path()).unwrap();

    let (records, issues) = collect_runtime_records(&paths, &[bound]);

    assert_eq!(records.len(), 1);
    assert_eq!(records[0].record(), &record);
    assert!(issues.finish().is_ok());
}

#[test]
fn scanned_conflict_cannot_replace_previously_bound_evidence() {
    let directory = tempfile::tempdir().unwrap();
    let paths = Paths::new(directory.path());
    let terminal_id = Uuid::new_v4();
    paths.ensure_session(terminal_id).unwrap();
    let trusted = fixture_record(&paths, terminal_id);
    let bound = bind_record(&paths, trusted.clone());
    let mut tampered = trusted.clone();
    tampered.auth_token = TAMPERED_AUTH_TOKEN.into();
    yolo_runtime::paths::atomic_write_json(&paths.runtime_record(terminal_id), &tampered).unwrap();

    let (records, issues) = collect_runtime_records(&paths, &[bound]);

    assert_eq!(records.len(), 1);
    assert_eq!(records[0].record(), &trusted);
    assert!(issues.finish().is_err());
}

#[test]
fn matching_adopted_generation_updates_bound_cleanup_authority() {
    let directory = tempfile::tempdir().unwrap();
    let paths = Paths::new(directory.path());
    let terminal_id = Uuid::new_v4();
    paths.ensure_session(terminal_id).unwrap();
    let trusted = fixture_record(&paths, terminal_id);
    let bound = bind_record(&paths, trusted.clone());
    let mut adopted = trusted.clone();
    adopted.controller_generation = ADOPTED_TEST_GENERATION;
    yolo_runtime::paths::atomic_write_json(&paths.runtime_record(terminal_id), &adopted).unwrap();

    let (records, issues) = collect_runtime_records(&paths, &[bound]);

    assert_eq!(records.len(), 1);
    assert_eq!(
        records[0].record().controller_generation,
        ADOPTED_TEST_GENERATION
    );
    assert_eq!(records[0].record().auth_token, trusted.auth_token);
    assert!(issues.finish().is_ok());
}

#[test]
fn non_session_entries_are_ignored_while_uuid_symlinks_are_rejected() {
    use std::os::unix::fs::symlink;

    let directory = tempfile::tempdir().unwrap();
    let outside = tempfile::tempdir().unwrap();
    let paths = Paths::new(directory.path());
    let valid_session = Uuid::new_v4();
    let symlink_session = Uuid::new_v4();
    paths.ensure_session(valid_session).unwrap();
    let valid = fixture_record(&paths, valid_session);
    yolo_runtime::paths::atomic_write_json(&paths.runtime_record(valid_session), &valid).unwrap();
    std::fs::create_dir(paths.sessions_dir().join(NON_SESSION_ENTRY)).unwrap();
    symlink(outside.path(), paths.session_dir(symlink_session)).unwrap();

    let scan = readable_runtime_records(&paths);

    assert_eq!(scan.records, [valid]);
    assert_eq!(scan.failures.len(), 1);
}

#[test]
fn schema_and_boot_mismatches_are_rejected() {
    let directory = tempfile::tempdir().unwrap();
    let paths = Paths::new(directory.path());
    let terminal_id = Uuid::new_v4();
    let mut record = fixture_record(&paths, terminal_id);
    record.schema_version = INVALID_SCHEMA_VERSION;
    assert!(validate_runtime_record(&paths, &record, Some(terminal_id)).is_err());
    record.schema_version = yolo_runtime::constants::RUNTIME_RECORD_SCHEMA_VERSION;
    record.boot_id = FOREIGN_BOOT_ID.into();
    assert!(validate_runtime_record(&paths, &record, Some(terminal_id)).is_err());
}

fn fixture_record(paths: &Paths, terminal_id: Uuid) -> RuntimeRecord {
    let process_id = std::process::id();
    let start_time = yolo_process_test_support::process_start_time(process_id).unwrap();
    RuntimeRecord {
        schema_version: yolo_runtime::constants::RUNTIME_RECORD_SCHEMA_VERSION,
        terminal_id: terminal_id,
        runtime_epoch: Uuid::new_v4(),
        worker_id: Uuid::new_v4(),
        worker_pid: process_id,
        worker_start_time: start_time,
        child_pid: process_id,
        child_start_time: start_time,
        boot_id: yolo_process_test_support::boot_id(),
        socket_path: paths.worker_socket(terminal_id).unwrap(),
        auth_token: TEST_AUTH_TOKEN.into(),
        controller_generation: TEST_GENERATION,
        started_at_ms: TEST_STARTED_AT_MS,
        command: TEST_COMMAND.into(),
        args: Vec::new(),
        cwd: paths.home().to_path_buf(),
        cols: TEST_COLUMNS,
        rows: TEST_ROWS,
        bootstrap_delivery: yolo_runtime::model::BootstrapDelivery::NotRequired,
    }
}

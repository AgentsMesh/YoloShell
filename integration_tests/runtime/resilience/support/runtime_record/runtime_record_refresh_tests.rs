use super::cleanup::collect_runtime_records;
use super::runtime_record_test_support::{bind_record, record_for_identity};
use super::*;

const INITIAL_GENERATION: u64 = 1;
const ADOPTED_GENERATION: u64 = INITIAL_GENERATION + 1;

#[test]
fn matching_runtime_refreshes_only_mutable_fields() {
    let directory = tempfile::tempdir().unwrap();
    let paths = Paths::new(directory.path());
    let identity = ProcessIdentity::capture(std::process::id()).unwrap();
    let trusted = record_for_identity(&paths, Uuid::new_v4(), identity);
    let bound = bind_record(&paths, trusted.clone());
    let mut refreshed = trusted.clone();
    refreshed.controller_generation = ADOPTED_GENERATION;
    refreshed.cols = yolo_runtime::model::MIN_TERMINAL_COLUMNS;
    refreshed.rows = yolo_runtime::model::MIN_TERMINAL_ROWS;
    refreshed.bootstrap_delivery = yolo_runtime::model::BootstrapDelivery::Delivered;
    yolo_runtime::paths::atomic_write_json(&paths.runtime_record(refreshed.terminal_id), &refreshed)
        .unwrap();
    assert!(bound.verify_runtime_file().is_err());

    let (records, issues) = collect_runtime_records(&paths, &[bound]);

    assert_eq!(records.len(), 1);
    assert_eq!(records[0].record(), &refreshed);
    records[0].verify_runtime_file().unwrap();
    assert!(issues.finish().is_ok());
}

#[test]
fn matching_runtime_rejects_controller_generation_rollback() {
    let directory = tempfile::tempdir().unwrap();
    let paths = Paths::new(directory.path());
    let identity = ProcessIdentity::capture(std::process::id()).unwrap();
    let mut trusted = record_for_identity(&paths, Uuid::new_v4(), identity);
    trusted.controller_generation = ADOPTED_GENERATION;
    let bound = bind_record(&paths, trusted.clone());
    let mut stale = trusted.clone();
    stale.controller_generation = INITIAL_GENERATION;
    yolo_runtime::paths::atomic_write_json(&paths.runtime_record(stale.terminal_id), &stale)
        .unwrap();

    let (records, issues) = collect_runtime_records(&paths, &[bound]);

    assert_eq!(records.len(), 1);
    assert_eq!(records[0].record(), &trusted);
    assert!(issues.finish().is_err());
}

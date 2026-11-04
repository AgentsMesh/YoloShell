use super::runtime_record_test_support::{record_for_identity, session_view};
use super::*;
use std::collections::HashSet;

const BINDING_TEST_EXECUTABLE: &str = "/bin/sleep";
const BINDING_TEST_ARGUMENT: &str = "60";
const RESCAN_GENERATION_INCREMENT: u64 = 1;
const SIGNAL_SETTLEMENT_DELAY: Duration = Duration::from_millis(100);

#[test]
fn failed_tree_capture_grants_no_worker_root_or_successor_authority() {
    let directory = tempfile::tempdir().unwrap();
    let paths = Paths::new(directory.path());
    let (mut cleanup, capture) = RuntimeCleanup::capture(&paths);
    capture.unwrap();
    let mut worker = spawn_sleep();
    let worker_identity = ProcessIdentity::capture(worker.id()).unwrap();
    let mut root = spawn_sleep();
    let root_identity = ProcessIdentity::capture(root.id()).unwrap();
    let terminal_id = Uuid::new_v4();
    let mut record = record_for_identity(&paths, terminal_id, worker_identity);
    record.child_pid = root_identity.pid;
    record.child_start_time = root_identity.start_time;
    yolo_runtime::paths::atomic_write_json(&paths.runtime_record(terminal_id), &record).unwrap();
    let view = session_view(&record);
    root.kill().unwrap();
    root.wait().unwrap();

    assert!(cleanup.bind_view(&paths, &view).is_err());
    let mut successor = spawn_sleep();
    let successor_identity = ProcessIdentity::capture(successor.id()).unwrap();
    cleanup.request();
    cleanup.force();
    assert!(cleanup.finish().is_err());
    assert!(worker_identity.matches());
    assert!(successor_identity.matches());

    worker.kill().unwrap();
    worker.wait().unwrap();
    successor.kill().unwrap();
    successor.wait().unwrap();
}

#[test]
fn partial_root_capture_never_signals_the_roots_already_captured() {
    let mut first = spawn_sleep();
    let first_identity = yolo_process_test_support::ProcessIdentity::capture(first.id()).unwrap();
    let mut second = spawn_sleep();
    let second_identity = yolo_process_test_support::ProcessIdentity::capture(second.id()).unwrap();
    second.kill().unwrap();
    second.wait().unwrap();

    let mut trees = cleanup_tree::RuntimeProcessTrees::default();
    let roots = HashSet::from([first_identity, second_identity]);
    assert!(trees.signal_roots(&roots, libc::SIGTERM).is_err());
    std::thread::sleep(SIGNAL_SETTLEMENT_DELAY);
    assert!(first_identity.matches());

    first.kill().unwrap();
    first.wait().unwrap();
}

#[test]
fn boot_discovery_requires_authenticated_binding_before_signal_authority() {
    let directory = tempfile::tempdir().unwrap();
    let paths = Paths::new(directory.path());
    let mut victim = spawn_sleep();
    let identity = ProcessIdentity::capture(victim.id()).unwrap();
    let record = record_for_identity(&paths, Uuid::new_v4(), identity);
    let runtime_identity = cleanup_tree::runtime_child_identity(&record);
    yolo_runtime::paths::atomic_write_json(&paths.runtime_record(record.terminal_id), &record)
        .unwrap();

    let (mut cleanup, capture) = RuntimeCleanup::capture_for_boot(&paths, &record.boot_id);
    capture.unwrap();
    assert!(cleanup.bound_records_for_test().is_empty());
    assert_eq!(cleanup.pending_records_for_test(), [record.clone()]);
    assert!(cleanup.tree_identities_for_test().is_empty());

    cleanup
        .bind_view_for_boot(&paths, &session_view(&record), &record.boot_id)
        .unwrap();
    assert_eq!(cleanup.bound_records_for_test(), [record]);
    assert!(cleanup.pending_records_for_test().is_empty());
    assert!(
        cleanup
            .tree_identities_for_test()
            .contains(&runtime_identity)
    );

    victim.kill().unwrap();
    victim.wait().unwrap();
}

#[test]
fn failed_rescan_preserves_records_and_tree_custody_as_one_snapshot() {
    let directory = tempfile::tempdir().unwrap();
    let paths = Paths::new(directory.path());
    let mut trusted = spawn_sleep();
    let trusted_identity = ProcessIdentity::capture(trusted.id()).unwrap();
    let trusted_record = record_for_identity(&paths, Uuid::new_v4(), trusted_identity);
    yolo_runtime::paths::atomic_write_json(
        &paths.runtime_record(trusted_record.terminal_id),
        &trusted_record,
    )
    .unwrap();
    let (mut cleanup, capture) = RuntimeCleanup::capture(&paths);
    assert!(capture.is_err());
    cleanup
        .bind_view(&paths, &session_view(&trusted_record))
        .unwrap();
    let before_records = cleanup.bound_records_for_test();
    let before_trees = cleanup.tree_identities_for_test();

    let mut refreshed = trusted_record.clone();
    refreshed.controller_generation += RESCAN_GENERATION_INCREMENT;
    yolo_runtime::paths::atomic_write_json(&paths.runtime_record(refreshed.terminal_id), &refreshed)
        .unwrap();
    let mut orphan = spawn_sleep();
    let orphan_identity = ProcessIdentity::capture(orphan.id()).unwrap();
    let orphan_record = record_for_identity(&paths, Uuid::new_v4(), orphan_identity);
    let orphan_runtime_identity = cleanup_tree::runtime_child_identity(&orphan_record);
    yolo_runtime::paths::atomic_write_json(
        &paths.runtime_record(orphan_record.terminal_id),
        &orphan_record,
    )
    .unwrap();

    assert!(cleanup.rescan(&paths).is_err());
    assert_eq!(cleanup.bound_records_for_test(), before_records);
    assert_eq!(cleanup.tree_identities_for_test(), before_trees);
    assert!(
        !cleanup
            .tree_identities_for_test()
            .contains(&orphan_runtime_identity)
    );

    trusted.kill().unwrap();
    trusted.wait().unwrap();
    orphan.kill().unwrap();
    orphan.wait().unwrap();
}

#[test]
fn missing_pending_record_remains_fail_closed_without_tree_authority() {
    let directory = tempfile::tempdir().unwrap();
    let paths = Paths::new(directory.path());
    let mut victim = spawn_sleep();
    let identity = ProcessIdentity::capture(victim.id()).unwrap();
    let record = record_for_identity(&paths, Uuid::new_v4(), identity);
    yolo_runtime::paths::atomic_write_json(&paths.runtime_record(record.terminal_id), &record)
        .unwrap();
    let (mut cleanup, capture) = RuntimeCleanup::capture_for_boot(&paths, &record.boot_id);
    capture.unwrap();
    std::fs::remove_file(paths.runtime_record(record.terminal_id)).unwrap();

    assert!(cleanup.rescan_for_boot(&paths, &record.boot_id).is_err());
    assert_eq!(cleanup.pending_records_for_test(), [record]);
    assert!(cleanup.bound_records_for_test().is_empty());
    assert!(cleanup.tree_identities_for_test().is_empty());
    assert!(identity.matches());

    victim.kill().unwrap();
    victim.wait().unwrap();
}

fn spawn_sleep() -> ChildGuard {
    ChildGuard::new(
        Command::new(BINDING_TEST_EXECUTABLE)
            .arg(BINDING_TEST_ARGUMENT)
            .spawn()
            .unwrap(),
    )
}

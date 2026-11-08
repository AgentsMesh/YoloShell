use crate::worker::tests::*;
const WORKER_LAUNCH_CLEANUP_CYCLES: usize = 4;
const MAX_WORKER_LAUNCH_RECYCLE_SLOTS: usize = 2;
const SAVED_WORKER_LAUNCH_FILE_NAME: &str = "saved-worker-launch";

#[test]
fn worker_launch_cleanup_preserves_a_path_replacement() {
    let directory = tempfile::tempdir().unwrap();
    let paths = Paths::new(directory.path());
    let terminal_id = Uuid::new_v4();
    let original = launch_spec(&paths, terminal_id);
    let mut replacement = launch_spec(&paths, terminal_id);
    replacement.worker_id = Uuid::new_v4();
    write_launch(&paths, terminal_id, &original);
    let path = paths.worker_launch(terminal_id);
    let saved = paths
        .session_dir(terminal_id)
        .join(SAVED_WORKER_LAUNCH_FILE_NAME);
    let entry = crate::paths::private_file::PrivateFileEntry::open(&path).unwrap();

    let result = crate::paths::test_support::remove_private_file_with_hook(&entry, || {
        std::fs::rename(&path, &saved)?;
        atomic_write_json(&path, &replacement)
    });

    assert!(result.is_err());
    let current: WorkerLaunchSpec = read_json(&path).unwrap();
    let displaced: WorkerLaunchSpec = read_json(&saved).unwrap();
    assert_eq!(current.worker_id, replacement.worker_id);
    assert_eq!(displaced.worker_id, original.worker_id);
}

#[test]
fn worker_launch_cleanup_reuses_a_bounded_number_of_slots() {
    let directory = tempfile::tempdir().unwrap();
    let paths = Paths::new(directory.path());
    let terminal_id = Uuid::new_v4();
    paths.ensure_session(terminal_id).unwrap();
    let session_directory = paths.session_dir(terminal_id);
    for _ in 0..WORKER_LAUNCH_CLEANUP_CYCLES {
        write_launch(&paths, terminal_id, &launch_spec(&paths, terminal_id));
        crate::paths::private_file::remove_private_file(&paths.worker_launch(terminal_id)).unwrap();
        assert!(!paths.worker_launch(terminal_id).exists());
        assert!(recycle_slot_count(&session_directory) <= MAX_WORKER_LAUNCH_RECYCLE_SLOTS);
    }
}

fn recycle_slot_count(directory: &Path) -> usize {
    matching_slot_count(
        directory,
        crate::paths::test_support::staging_artifact_prefix(),
    ) + matching_slot_count(
        directory,
        crate::paths::test_support::retired_artifact_prefix(),
    )
}

fn matching_slot_count(directory: &Path, prefix: &str) -> usize {
    std::fs::read_dir(directory)
        .unwrap()
        .flatten()
        .filter(|entry| entry.file_name().to_string_lossy().starts_with(prefix))
        .count()
}

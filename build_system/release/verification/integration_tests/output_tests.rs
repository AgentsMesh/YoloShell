use super::support::{ARCHIVE_BYTES, error_text, fifo};
use super::test_constants::{
    ABRUPT_OUTPUT_ROOT_ENVIRONMENT, ABRUPT_OUTPUT_TEST_FILTER, OUTPUT_TEST_DESTINATION_NAME,
};
use crate::output::{OutputDirectory, OutputTransaction};
use crate::output_stage::StagedCandidate;
use crate::test_support::{OutputCleanup, evidence, seal};
use std::fs;
use std::os::unix::fs::{PermissionsExt, symlink};
use std::path::Path;
use std::process::Command;
use std::sync::{Arc, Barrier};
use yolo_release_candidate_artifact::PinnedArtifact;
use yolo_release_contracts::{
    ARCHIVE_NAME, CHECKSUM_NAME, PERMISSION_BITS_MASK, PROVENANCE_NAME, READ_ONLY_FILE_MODE,
};
use yolo_release_filesystem::DirectoryHandle;

#[test]
fn fixed_candidate_is_installed_as_one_read_only_directory() {
    let root = tempfile::tempdir().unwrap();
    let output_path = root.path().join("output");
    let artifact = PinnedArtifact::fixture(root.path(), ARCHIVE_BYTES).unwrap();
    let (checksum, evidence) = evidence();
    let output = OutputTransaction::prepare(&output_path)
        .unwrap()
        .publish(&artifact, &checksum, &evidence)
        .unwrap();
    let _cleanup = OutputCleanup::new(&output_path);
    assert_eq!(
        fs::read(output_path.join(ARCHIVE_NAME)).unwrap(),
        ARCHIVE_BYTES
    );
    assert_eq!(output.read_checksum().unwrap(), checksum);
    assert_eq!(output.read_provenance().unwrap(), evidence);
    for name in [ARCHIVE_NAME, CHECKSUM_NAME, PROVENANCE_NAME] {
        assert_eq!(
            fs::metadata(output_path.join(name))
                .unwrap()
                .permissions()
                .mode()
                & PERMISSION_BITS_MASK,
            READ_ONLY_FILE_MODE
        );
    }
}

#[test]
fn symlink_output_directory_is_rejected() {
    let root = tempfile::tempdir().unwrap();
    let real = root.path().join("real");
    let link = root.path().join("link");
    fs::create_dir(&real).unwrap();
    symlink(&real, &link).unwrap();
    assert!(error_text(OutputDirectory::open(&link)).contains("open candidate output"));
    assert!(error_text(OutputTransaction::prepare(&link)).contains("initially be absent"));
}

#[test]
fn failed_stage_leaves_candidate_name_absent() {
    let root = tempfile::tempdir().unwrap();
    let output_path = root.path().join("output");
    let artifact = PinnedArtifact::fixture(root.path(), ARCHIVE_BYTES).unwrap();
    assert!(
        OutputTransaction::prepare(&output_path)
            .unwrap()
            .publish(&artifact, b"invalid", b"invalid")
            .is_err()
    );
    assert!(!output_path.exists());
}

#[test]
fn concurrent_producers_install_exactly_one_complete_candidate() {
    let root = tempfile::tempdir().unwrap();
    let output_path = root.path().join("output");
    let first = root.path().join("first");
    let second = root.path().join("second");
    fs::create_dir(&first).unwrap();
    fs::create_dir(&second).unwrap();
    let transactions = [
        OutputTransaction::prepare(&output_path).unwrap(),
        OutputTransaction::prepare(&output_path).unwrap(),
    ];
    let artifacts = [
        PinnedArtifact::fixture(&first, ARCHIVE_BYTES).unwrap(),
        PinnedArtifact::fixture(&second, ARCHIVE_BYTES).unwrap(),
    ];
    let barrier = Arc::new(Barrier::new(transactions.len()));
    let (checksum, provenance) = evidence();
    let threads = transactions
        .into_iter()
        .zip(artifacts)
        .map(|(transaction, artifact)| {
            let barrier = Arc::clone(&barrier);
            let checksum = checksum.clone();
            let provenance = provenance.clone();
            std::thread::spawn(move || {
                barrier.wait();
                transaction
                    .publish(&artifact, &checksum, &provenance)
                    .is_ok()
            })
        })
        .collect::<Vec<_>>();
    let successes = threads
        .into_iter()
        .map(|thread| usize::from(thread.join().unwrap()))
        .sum::<usize>();
    assert_eq!(successes, 1);
    let _cleanup = OutputCleanup::new(&output_path);
    assert!(
        OutputDirectory::open(&output_path)
            .unwrap()
            .open_artifact()
            .is_ok()
    );
}

#[test]
fn competitor_created_after_prepare_cannot_be_replaced() {
    let root = tempfile::tempdir().unwrap();
    let output_path = root.path().join(OUTPUT_TEST_DESTINATION_NAME);
    let _transaction = OutputTransaction::prepare(&output_path).unwrap();
    let parent = DirectoryHandle::open(root.path()).unwrap();
    let mut stage = staged_candidate(root.path(), &parent);
    fs::create_dir(&output_path).unwrap();
    assert!(stage.install_name(OUTPUT_TEST_DESTINATION_NAME).is_err());
    drop(stage);
    assert_eq!(fs::read_dir(&output_path).unwrap().count(), 0);
}

#[test]
fn abrupt_exit_before_install_never_publishes_candidate() {
    if let Some(root) = std::env::var_os(ABRUPT_OUTPUT_ROOT_ENVIRONMENT) {
        let root = std::path::PathBuf::from(root);
        let parent = DirectoryHandle::open(&root).unwrap();
        let stage = staged_candidate(&root, &parent);
        std::mem::forget(stage);
        std::process::exit(0);
    }
    let root = tempfile::tempdir().unwrap();
    let status = Command::new(std::env::current_exe().unwrap())
        .arg(ABRUPT_OUTPUT_TEST_FILTER)
        .env(ABRUPT_OUTPUT_ROOT_ENVIRONMENT, root.path())
        .status()
        .unwrap();
    assert!(status.success());
    assert!(!root.path().join(OUTPUT_TEST_DESTINATION_NAME).exists());
}

#[test]
fn missing_fixed_output_is_rejected() {
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join(OUTPUT_TEST_DESTINATION_NAME);
    fs::create_dir(&path).unwrap();
    seal(&path).unwrap();
    let _cleanup = OutputCleanup::new(&path);
    let output = OutputDirectory::open(&path).unwrap();
    assert!(error_text(output.open_artifact()).contains("open candidate output"));
    assert!(!path.join(PROVENANCE_NAME).exists());
}

#[test]
fn fifo_output_is_rejected_without_blocking() {
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join(OUTPUT_TEST_DESTINATION_NAME);
    fs::create_dir(&path).unwrap();
    fifo(&path.join(ARCHIVE_NAME));
    seal(&path).unwrap();
    let _cleanup = OutputCleanup::new(&path);
    let output = OutputDirectory::open(&path).unwrap();
    assert!(error_text(output.open_artifact()).contains("must be a file"));
}

fn staged_candidate(root: &Path, parent: &DirectoryHandle) -> StagedCandidate {
    let artifact = PinnedArtifact::fixture(root, ARCHIVE_BYTES).unwrap();
    let (checksum, provenance) = evidence();
    let mut stage = StagedCandidate::create(parent).unwrap();
    stage.write(&artifact, &checksum, &provenance).unwrap();
    stage
}

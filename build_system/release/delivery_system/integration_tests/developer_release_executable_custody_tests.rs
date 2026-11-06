use super::developer_release_fixture_archive::{EXECUTABLE_BYTES, candidate_archive};
use super::developer_release_fixture_runner::FakeDeveloperRunner;
use crate::developer_release_constants::{
    DEVELOPER_RELEASE_EXECUTABLE_CHANGED_ERROR, SIGNED_ARCHIVE_PATHS,
};
use std::fs;
use std::os::unix::fs::symlink;
use yolo_release_archive_audit::{audit_bytes, model::ZipInventory};
use yolo_release_developer_release_system::developer_release_executables::ExecutableBaseline;
use yolo_release_developer_release_system::developer_release_files::copy_writable;
use yolo_release_filesystem::DirectoryHandle;

const EXTRA_LINK: &str = "external-executable-link";
const NORMALIZED_COPY: &str = "normalized-copy";
const FOREIGN_BYTES: &[u8] = b"foreign executable";

fn prepared() -> (tempfile::TempDir, DirectoryHandle, ZipInventory) {
    let root = tempfile::tempdir().unwrap();
    for relative in SIGNED_ARCHIVE_PATHS {
        let path = root.path().join(relative);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, EXECUTABLE_BYTES).unwrap();
    }
    let handle = DirectoryHandle::open(root.path()).unwrap();
    (root, handle, audit_bytes(&candidate_archive()).unwrap())
}

#[test]
fn executable_baseline_requires_a_matching_regular_inventory_entry() {
    let (_root, handle, mut inventory) = prepared();
    let runner = FakeDeveloperRunner::passing();
    let baseline = ExecutableBaseline::capture(&handle, &inventory, &runner).unwrap();
    baseline.verify(&handle, &runner).unwrap();
    let executable = inventory
        .entries
        .iter_mut()
        .find(|entry| SIGNED_ARCHIVE_PATHS.contains(&entry.path.as_str()))
        .unwrap();
    executable.is_directory = true;
    assert_eq!(
        ExecutableBaseline::capture(&handle, &inventory, &runner)
            .err()
            .unwrap()
            .to_string(),
        DEVELOPER_RELEASE_EXECUTABLE_CHANGED_ERROR
    );
}

#[test]
fn executable_baseline_rejects_missing_symlinked_directory_and_aliased_inputs() {
    let replacements: &[fn(&std::path::Path, &std::path::Path)] = &[
        |path, _| {
            fs::remove_file(path).unwrap();
        },
        |path, _| {
            fs::remove_file(path).unwrap();
            fs::create_dir(path).unwrap();
        },
        |path, extra| {
            fs::rename(path, extra).unwrap();
            symlink(extra, path).unwrap();
        },
        |path, extra| {
            fs::hard_link(path, extra).unwrap();
        },
    ];
    for replace in replacements {
        let (root, handle, inventory) = prepared();
        let path = root.path().join(SIGNED_ARCHIVE_PATHS.first().unwrap());
        replace(&path, &root.path().join(EXTRA_LINK));
        assert_eq!(
            ExecutableBaseline::capture(&handle, &inventory, &FakeDeveloperRunner::passing())
                .err()
                .unwrap()
                .to_string(),
            DEVELOPER_RELEASE_EXECUTABLE_CHANGED_ERROR
        );
    }
}

#[test]
fn private_signature_normalization_never_overwrites_an_existing_name() {
    let (root, handle, _) = prepared();
    let source = fs::File::open(root.path().join(SIGNED_ARCHIVE_PATHS.first().unwrap())).unwrap();
    let destination = root.path().join(NORMALIZED_COPY);
    fs::write(&destination, FOREIGN_BYTES).unwrap();
    assert!(copy_writable(&handle, NORMALIZED_COPY, &source).is_err());
    assert_eq!(fs::read(destination).unwrap(), FOREIGN_BYTES);
}

#[test]
fn protected_release_files_require_exclusive_creation_and_retained_reopen() {
    use std::io::Write;
    use yolo_release_delivery_foundation::delivery_files::ProtectedFile;
    let root = tempfile::tempdir().unwrap();
    let handle = DirectoryHandle::open(root.path()).unwrap();
    assert!(ProtectedFile::open(&handle, NORMALIZED_COPY).is_err());
    let protected = ProtectedFile::create(&handle, NORMALIZED_COPY, |file| {
        file.write_all(FOREIGN_BYTES)?;
        Ok(())
    })
    .unwrap();
    assert!(ProtectedFile::create(&handle, NORMALIZED_COPY, |_| Ok(())).is_err());
    assert!(
        protected
            .verify_contents(&handle, NORMALIZED_COPY, FOREIGN_BYTES)
            .is_ok()
    );
    assert!(
        protected
            .verify_contents(&handle, NORMALIZED_COPY, EXECUTABLE_BYTES)
            .is_err()
    );
    fs::remove_file(root.path().join(NORMALIZED_COPY)).unwrap();
    assert!(protected.verify_at(&handle, NORMALIZED_COPY).is_err());
}

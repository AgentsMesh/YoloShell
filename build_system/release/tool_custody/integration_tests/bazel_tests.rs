use super::constants::TEST_LIST_ARGUMENT;
use crate::{TrustedBazel, candidate_path};
use std::ffi::OsString;
use std::fs::{self, File};
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use yolo_release_contracts::{
    BAZEL_VERSION_ARGUMENT, NO_FILE_FLAGS, PINNED_BAZEL_CACHE_PATH, PINNED_BAZEL_SHA256,
    PINNED_BAZEL_VERSION, PRIVATE_DIRECTORY_MODE, SEALED_DIRECTORY_MODE,
};
use yolo_release_filesystem::DirectoryHandle;
use yolo_release_foundation::digest::{bytes_sha256, opened_file_sha256};

const FAKE_EXECUTABLE_MODE: u32 = 0o700;
const FAKE_BAZEL: &[u8] = b"#!/bin/bash\nprintf 'bazel 8.2.1\\n'\n";
const REPLACEMENT_BAZEL: &[u8] = b"#!/bin/bash\nprintf 'replacement\\n'\n";

#[test]
fn launcher_resolution_never_consults_path() {
    let explicit = OsString::from("/trusted/bazel");
    let home = OsString::from("/safe/home");
    assert_eq!(
        candidate_path(Some(explicit.clone()), Some(home.clone())).unwrap(),
        PathBuf::from(explicit)
    );
    assert_eq!(
        candidate_path(None, Some(home)).unwrap(),
        Path::new("/safe/home").join(PINNED_BAZEL_CACHE_PATH)
    );
    assert!(candidate_path(None, None).is_err());
}

#[test]
fn hostile_bazel_real_must_match_pinned_digest() {
    let root = tempfile::tempdir().unwrap();
    let fake = root.path().join("bazel");
    executable(&fake, FAKE_BAZEL);
    assert!(
        format!(
            "{:#}",
            TrustedBazel::validate_identity_for_test(
                &fake,
                PINNED_BAZEL_SHA256,
                PINNED_BAZEL_VERSION,
            )
            .unwrap_err()
        )
        .contains("digest mismatch")
    );
}

#[test]
fn source_replacement_cannot_switch_validated_launcher() {
    let root = tempfile::tempdir().unwrap();
    let source = root.path().join("bazel");
    executable(&source, FAKE_BAZEL);
    let launcher = TrustedBazel::validate_identity_for_test(
        &source,
        &bytes_sha256(FAKE_BAZEL),
        PINNED_BAZEL_VERSION,
    )
    .unwrap();
    fs::write(&source, REPLACEMENT_BAZEL).unwrap();
    let output = std::process::Command::new(launcher.path())
        .arg(BAZEL_VERSION_ARGUMENT)
        .output()
        .unwrap();
    assert_eq!(
        String::from_utf8(output.stdout).unwrap().trim(),
        PINNED_BAZEL_VERSION
    );
    assert_ne!(
        launcher.evidence().canonical_path,
        launcher.evidence().execution_path
    );
    launcher.ensure_unchanged().unwrap();
}

#[test]
fn private_launcher_path_replacement_is_rejected() {
    let root = tempfile::tempdir().unwrap();
    let source = root.path().join("bazel");
    executable(&source, FAKE_BAZEL);
    let launcher = TrustedBazel::validate_identity_for_test(
        &source,
        &bytes_sha256(FAKE_BAZEL),
        PINNED_BAZEL_VERSION,
    )
    .unwrap();
    let moved = launcher.path().with_extension("moved");
    assert_eq!(
        fs::rename(launcher.path(), &moved).unwrap_err().kind(),
        std::io::ErrorKind::PermissionDenied
    );
    launcher.ensure_unchanged().unwrap();
}

#[test]
fn private_launcher_directory_replacement_is_rejected() {
    let root = tempfile::tempdir().unwrap();
    let source = root.path().join("bazel");
    executable(&source, FAKE_BAZEL);
    let launcher = TrustedBazel::validate_identity_for_test(
        &source,
        &bytes_sha256(FAKE_BAZEL),
        PINNED_BAZEL_VERSION,
    )
    .unwrap();
    let directory = launcher.path().parent().unwrap().to_owned();
    let moved = directory.with_extension("moved");
    assert_eq!(
        fs::rename(&directory, &moved).unwrap_err().kind(),
        std::io::ErrorKind::PermissionDenied
    );
    launcher.ensure_unchanged().unwrap();
}

#[test]
fn temporary_unseal_and_restored_path_is_rejected() {
    let root = tempfile::tempdir().unwrap();
    let source = root.path().join("bazel");
    executable(&source, FAKE_BAZEL);
    let launcher = TrustedBazel::validate_identity_for_test(
        &source,
        &bytes_sha256(FAKE_BAZEL),
        PINNED_BAZEL_VERSION,
    )
    .unwrap();
    let directory = DirectoryHandle::open(launcher.path().parent().unwrap()).unwrap();
    directory.set_flags(NO_FILE_FLAGS).unwrap();
    directory.set_mode(PRIVATE_DIRECTORY_MODE).unwrap();
    let moved = launcher.path().with_extension("moved");
    fs::rename(launcher.path(), &moved).unwrap();
    executable(launcher.path(), REPLACEMENT_BAZEL);
    fs::remove_file(launcher.path()).unwrap();
    fs::rename(&moved, launcher.path()).unwrap();
    directory.set_mode(SEALED_DIRECTORY_MODE).unwrap();
    directory.set_flags(libc::UF_IMMUTABLE).unwrap();
    assert!(
        format!("{:#}", launcher.ensure_unchanged().unwrap_err())
            .contains("sealed directory changed")
    );
}

#[test]
fn real_mach_o_executes_after_writable_copy_descriptor_is_closed() {
    let executable = std::env::current_exe().unwrap();
    let expected_output = std::process::Command::new(&executable)
        .arg(TEST_LIST_ARGUMENT)
        .output()
        .unwrap();
    assert!(expected_output.status.success());
    let stdout = String::from_utf8(expected_output.stdout).unwrap();
    let expected_version = stdout.strip_suffix('\n').unwrap().to_owned();
    let source = File::open(&executable).unwrap();
    let digest = opened_file_sha256(&source).unwrap();
    let launcher = TrustedBazel::validate_identity_with_argument_for_test(
        &executable,
        &digest,
        TEST_LIST_ARGUMENT,
        &expected_version,
    )
    .expect("real Mach-O version check must not deadlock");
    assert_eq!(launcher.evidence().version, expected_version);
}

#[test]
fn fixture_launcher_records_pinned_identity() {
    let launcher = TrustedBazel::fixture(PathBuf::from("/private/tmp/bazel"));
    assert_eq!(launcher.evidence().sha256, PINNED_BAZEL_SHA256);
}

fn executable(path: &Path, bytes: &[u8]) {
    fs::write(path, bytes).unwrap();
    fs::set_permissions(path, fs::Permissions::from_mode(FAKE_EXECUTABLE_MODE)).unwrap();
}

use super::support::error_text;
use super::test_constants::{
    ACL_ADD_ARGUMENT, ACL_REMOVE_ARGUMENT, CHMOD_EXECUTABLE, OUTPUT_EXTERNAL_LINK_NAME,
    OUTPUT_EXTRA_NAME, OUTPUT_REBOUND_NAME, OUTPUT_REPLACED_BACKUP_NAME,
    OUTPUT_TEST_DESTINATION_NAME, TAMPERED_OUTPUT_BYTES, TEST_DENY_DELETE_ACL,
};
use crate::output::OutputDirectory;
use crate::test_support::{OutputCleanup, publish, seal, unseal};
use crate::verify::{VerifyRequest, verify};
use std::collections::BTreeSet;
use std::fs::{self, File};
use std::os::fd::AsRawFd;
use std::os::unix::fs::{MetadataExt, PermissionsExt};
use std::process::Command;
use yolo_release_contracts::{
    ARCHIVE_NAME, OUTPUT_FILE_NAMES, PERMISSION_BITS_MASK, READ_ONLY_FILE_MODE,
    SEALED_DIRECTORY_MODE,
};
use yolo_release_filesystem::ensure_no_extended_acl;

#[test]
fn published_namespace_has_exact_modes_flags_links_and_acls() {
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join(OUTPUT_TEST_DESTINATION_NAME);
    let output = publish(root.path(), &path);
    let _cleanup = OutputCleanup::new(&path);
    let names = fs::read_dir(&path)
        .unwrap()
        .map(|entry| entry.unwrap().file_name().into_string().unwrap())
        .collect::<BTreeSet<_>>();
    assert_eq!(
        names,
        OUTPUT_FILE_NAMES.iter().map(ToString::to_string).collect()
    );
    assert_eq!(
        output.directory.mode().unwrap() & PERMISSION_BITS_MASK,
        SEALED_DIRECTORY_MODE
    );
    assert_eq!(output.directory.flags().unwrap(), libc::UF_IMMUTABLE);
    ensure_no_extended_acl(output.directory.raw_fd(), "tested output").unwrap();
    for &name in OUTPUT_FILE_NAMES {
        let file = File::open(path.join(name)).unwrap();
        let metadata = file.metadata().unwrap();
        assert_eq!(metadata.mode() & PERMISSION_BITS_MASK, READ_ONLY_FILE_MODE);
        assert_eq!(metadata.uid(), unsafe { libc::geteuid() });
        assert_eq!(metadata.nlink(), 1);
        ensure_no_extended_acl(file.as_raw_fd(), "tested output file").unwrap();
    }
}

#[test]
fn opened_destination_rebind_is_rejected_before_verification_returns() {
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join(OUTPUT_TEST_DESTINATION_NAME);
    let output = publish(root.path(), &path);
    let opened = output.open_candidate().unwrap();
    unseal(&path).unwrap();
    fs::rename(&path, root.path().join(OUTPUT_REBOUND_NAME)).unwrap();
    fs::create_dir(&path).unwrap();
    let error = error_text(output.verify_candidate(&opened));
    assert!(error.contains("sealed directory changed"), "{error}");
}

#[test]
fn every_fixed_entry_replacement_is_rejected_after_open() {
    for &name in OUTPUT_FILE_NAMES {
        let root = tempfile::tempdir().unwrap();
        let path = root.path().join(OUTPUT_TEST_DESTINATION_NAME);
        let output = publish(root.path(), &path);
        let _cleanup = OutputCleanup::new(&path);
        let opened = output.open_candidate().unwrap();
        unseal(&path).unwrap();
        let backup = path.join(OUTPUT_REPLACED_BACKUP_NAME);
        fs::rename(path.join(name), &backup).unwrap();
        fs::write(path.join(name), TAMPERED_OUTPUT_BYTES).unwrap();
        fs::set_permissions(
            path.join(name),
            fs::Permissions::from_mode(READ_ONLY_FILE_MODE),
        )
        .unwrap();
        seal(&path).unwrap();
        let error = error_text(output.verify_candidate(&opened));
        assert!(
            error.contains("candidate file") && error.contains("changed"),
            "{name}: {error}"
        );
    }
}

#[test]
fn extra_entry_and_external_hard_link_are_rejected() {
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join(OUTPUT_TEST_DESTINATION_NAME);
    publish(root.path(), &path);
    let _cleanup = OutputCleanup::new(&path);
    unseal(&path).unwrap();
    fs::write(path.join(OUTPUT_EXTRA_NAME), TAMPERED_OUTPUT_BYTES).unwrap();
    seal(&path).unwrap();
    let error = error_text(verify(VerifyRequest {
        output_directory: &path,
    }));
    assert!(error.contains("entries mismatch"), "{error}");

    unseal(&path).unwrap();
    fs::remove_file(path.join(OUTPUT_EXTRA_NAME)).unwrap();
    fs::hard_link(
        path.join(ARCHIVE_NAME),
        root.path().join(OUTPUT_EXTERNAL_LINK_NAME),
    )
    .unwrap();
    seal(&path).unwrap();
    let error = error_text(verify(VerifyRequest {
        output_directory: &path,
    }));
    assert!(error.contains("must have one link"), "{error}");
}

#[test]
fn extended_acl_on_candidate_file_is_rejected() {
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join(OUTPUT_TEST_DESTINATION_NAME);
    publish(root.path(), &path);
    let _cleanup = OutputCleanup::new(&path);
    unseal(&path).unwrap();
    let file = path.join(ARCHIVE_NAME);
    chmod(&[ACL_ADD_ARGUMENT, TEST_DENY_DELETE_ACL], &file);
    seal(&path).unwrap();
    let error = error_text(OutputDirectory::open(&path).and_then(|output| output.open_artifact()));
    assert!(error.contains("must not have an ACL"), "{error}");
    unseal(&path).unwrap();
    chmod(&[ACL_REMOVE_ARGUMENT], &file);
    seal(&path).unwrap();
}

fn chmod(arguments: &[&str], path: &std::path::Path) {
    let status = Command::new(CHMOD_EXECUTABLE)
        .args(arguments)
        .arg(path)
        .status()
        .unwrap();
    assert!(status.success());
}

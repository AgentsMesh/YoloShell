use super::support::{ARCHIVE_BYTES, error_text};
use super::test_constants::{
    ACL_ADD_ARGUMENT, ACL_REMOVE_ARGUMENT, CHMOD_EXECUTABLE, INJECTED_INITIAL_SYNC_FAILURE,
    INJECTED_PUBLICATION_FAILURE, OUTPUT_TEST_DESTINATION_NAME, TEST_DENY_DELETE_ACL,
    UNSAFE_OUTPUT_PARENT_MODE,
};
use crate::output::OutputTransaction;
use crate::output_publish::{PublicationCheckpoint, PublicationSync, PublishHooks};
use crate::test_support::{assert_no_staging_name, evidence};
use anyhow::{Result, bail};
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::process::Command;
use yolo_release_candidate_artifact::PinnedArtifact;
use yolo_release_contracts::PRIVATE_DIRECTORY_MODE;
use yolo_release_filesystem::DirectoryHandle;

#[test]
fn parent_write_policy_is_rechecked_after_prepare() {
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join(OUTPUT_TEST_DESTINATION_NAME);
    let transaction = OutputTransaction::prepare(&path).unwrap();
    let artifact = PinnedArtifact::fixture(root.path(), ARCHIVE_BYTES).unwrap();
    let (checksum, provenance) = evidence();
    fs::set_permissions(
        root.path(),
        fs::Permissions::from_mode(UNSAFE_OUTPUT_PARENT_MODE),
    )
    .unwrap();
    let error = error_text(transaction.publish(&artifact, &checksum, &provenance));
    assert!(
        error.contains("must not be group or world writable"),
        "{error}"
    );
    assert!(!path.exists());
    fs::set_permissions(
        root.path(),
        fs::Permissions::from_mode(PRIVATE_DIRECTORY_MODE),
    )
    .unwrap();
}

#[test]
fn parent_with_extended_acl_is_rejected() {
    let root = tempfile::tempdir().unwrap();
    chmod(&[ACL_ADD_ARGUMENT, TEST_DENY_DELETE_ACL], root.path());
    let path = root.path().join(OUTPUT_TEST_DESTINATION_NAME);
    let error = error_text(OutputTransaction::prepare(&path));
    assert!(error.contains("must not have an ACL"), "{error}");
    chmod(&[ACL_REMOVE_ARGUMENT], root.path());
}

#[test]
fn failure_after_precommit_verification_removes_formal_candidate_name() {
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join(OUTPUT_TEST_DESTINATION_NAME);
    let transaction = OutputTransaction::prepare(&path).unwrap();
    let artifact = PinnedArtifact::fixture(root.path(), ARCHIVE_BYTES).unwrap();
    let (checksum, provenance) = evidence();
    let error = error_text(transaction.publish_with_hooks(
        &artifact,
        &checksum,
        &provenance,
        &FailAtCheckpoint(PublicationCheckpoint::ModeProtected),
    ));
    assert!(error.contains(INJECTED_PUBLICATION_FAILURE), "{error}");
    assert!(!path.exists());
    assert_no_staging_name(root.path());
}

#[test]
fn parent_sync_failure_after_rename_rolls_back() {
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join(OUTPUT_TEST_DESTINATION_NAME);
    let transaction = OutputTransaction::prepare(&path).unwrap();
    let artifact = PinnedArtifact::fixture(root.path(), ARCHIVE_BYTES).unwrap();
    let (checksum, provenance) = evidence();
    let error = error_text(transaction.publish_with_hooks(
        &artifact,
        &checksum,
        &provenance,
        &FailParentSync(PublicationSync::Initial, INJECTED_INITIAL_SYNC_FAILURE),
    ));
    assert!(error.contains(INJECTED_INITIAL_SYNC_FAILURE), "{error}");
    assert!(!path.exists());
    assert_no_staging_name(root.path());
}

struct FailAtCheckpoint(PublicationCheckpoint);

impl PublishHooks for FailAtCheckpoint {
    fn checkpoint(&self, checkpoint: PublicationCheckpoint) -> Result<()> {
        if checkpoint == self.0 {
            bail!(INJECTED_PUBLICATION_FAILURE);
        }
        Ok(())
    }
}

struct FailParentSync(PublicationSync, &'static str);

impl PublishHooks for FailParentSync {
    fn sync_parent(&self, phase: PublicationSync, parent: &DirectoryHandle) -> Result<()> {
        if phase == self.0 {
            bail!(self.1);
        }
        parent.durability_barrier()
    }
}

fn chmod(arguments: &[&str], path: &std::path::Path) {
    let status = Command::new(CHMOD_EXECUTABLE)
        .args(arguments)
        .arg(path)
        .status()
        .unwrap();
    assert!(status.success());
}

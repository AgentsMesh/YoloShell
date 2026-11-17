use super::support::error_text;
use super::test_constants::{
    INJECTED_ACKNOWLEDGEMENT_FAILURE, OUTPUT_REBOUND_NAME, OUTPUT_TEST_DESTINATION_NAME,
    TAMPERED_OUTPUT_BYTES,
};
use crate::output_publish::{PublicationCheckpoint, PublishHooks};
use crate::test_support::{OutputCleanup, publish_with_hooks, seal, unseal};
use crate::verify::{VerifyRequest, verify};
use anyhow::{Result, bail};
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use yolo_release_contracts::{
    COMMITTED_ACKNOWLEDGEMENT_ERROR, COMMITTED_VALIDATION_ERROR, PRIVATE_FILE_MODE,
    PROVENANCE_NAME, READ_ONLY_FILE_MODE,
};

#[test]
fn acknowledgement_failure_preserves_a_verified_candidate() {
    for checkpoint in [
        PublicationCheckpoint::DurabilityBarriersComplete,
        PublicationCheckpoint::CommitMarked,
        PublicationCheckpoint::FinalValidationComplete,
    ] {
        assert_verified_acknowledgement_failure(checkpoint);
    }
}

fn assert_verified_acknowledgement_failure(checkpoint: PublicationCheckpoint) {
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join(OUTPUT_TEST_DESTINATION_NAME);
    let error = error_text(publish_with_hooks(
        root.path(),
        &path,
        &FailAcknowledgement(checkpoint),
    ));
    let _cleanup = OutputCleanup::new(&path);
    assert!(error.contains(INJECTED_ACKNOWLEDGEMENT_FAILURE), "{error}");
    assert!(error.contains(COMMITTED_ACKNOWLEDGEMENT_ERROR), "{error}");
    assert!(!error.contains(COMMITTED_VALIDATION_ERROR), "{error}");
    verify(VerifyRequest {
        output_directory: &path,
    })
    .unwrap();
}

#[test]
fn failed_acknowledgement_with_mutation_is_a_validation_failure() {
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join(OUTPUT_TEST_DESTINATION_NAME);
    let hooks = TamperAndFailAcknowledgement(path.clone());
    let error = error_text(publish_with_hooks(root.path(), &path, &hooks));
    let _cleanup = OutputCleanup::new(&path);
    assert_validation_failure(&error);
    assert!(
        verify(VerifyRequest {
            output_directory: &path,
        })
        .is_err()
    );
}

#[test]
fn postcommit_path_rebind_is_a_validation_failure() {
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join(OUTPUT_TEST_DESTINATION_NAME);
    let rebound = root.path().join(OUTPUT_REBOUND_NAME);
    let hooks = RebindAfterCommit {
        path: path.clone(),
        rebound: rebound.clone(),
    };
    let error = error_text(publish_with_hooks(root.path(), &path, &hooks));
    let _rebound_cleanup = OutputCleanup::new(&rebound);
    let _output_cleanup = OutputCleanup::new(&path);
    assert_validation_failure(&error);
    assert!(
        verify(VerifyRequest {
            output_directory: &path,
        })
        .is_err()
    );
}

struct FailAcknowledgement(PublicationCheckpoint);

impl PublishHooks for FailAcknowledgement {
    fn checkpoint(&self, checkpoint: PublicationCheckpoint) -> Result<()> {
        if checkpoint == self.0 {
            bail!(INJECTED_ACKNOWLEDGEMENT_FAILURE);
        }
        Ok(())
    }
}

struct TamperAndFailAcknowledgement(PathBuf);

impl PublishHooks for TamperAndFailAcknowledgement {
    fn checkpoint(&self, checkpoint: PublicationCheckpoint) -> Result<()> {
        if checkpoint == PublicationCheckpoint::FinalValidationComplete {
            tamper_provenance(&self.0)?;
            bail!(INJECTED_ACKNOWLEDGEMENT_FAILURE);
        }
        Ok(())
    }
}

struct RebindAfterCommit {
    path: PathBuf,
    rebound: PathBuf,
}

impl PublishHooks for RebindAfterCommit {
    fn checkpoint(&self, checkpoint: PublicationCheckpoint) -> Result<()> {
        if checkpoint == PublicationCheckpoint::CommitMarked {
            unseal(&self.path)?;
            fs::rename(&self.path, &self.rebound)?;
            fs::create_dir(&self.path)?;
            seal(&self.path)?;
        }
        Ok(())
    }
}

fn tamper_provenance(path: &Path) -> Result<()> {
    unseal(path)?;
    let provenance = path.join(PROVENANCE_NAME);
    fs::set_permissions(&provenance, fs::Permissions::from_mode(PRIVATE_FILE_MODE))?;
    fs::write(&provenance, TAMPERED_OUTPUT_BYTES)?;
    fs::set_permissions(&provenance, fs::Permissions::from_mode(READ_ONLY_FILE_MODE))?;
    seal(path)?;
    Ok(())
}

fn assert_validation_failure(error: &str) {
    assert!(error.contains(COMMITTED_VALIDATION_ERROR), "{error}");
    assert!(!error.contains(COMMITTED_ACKNOWLEDGEMENT_ERROR), "{error}");
}

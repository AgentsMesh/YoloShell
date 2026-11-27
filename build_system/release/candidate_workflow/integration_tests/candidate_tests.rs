use crate::{CandidateExecuting, CandidateExecution, CandidateRequest, create_with_executor};
use anyhow::{Result, bail};
use std::fs;
use std::path::{Path, PathBuf};
use yolo_release_candidate_artifact::PinnedArtifact;
use yolo_release_contracts::{ARCHIVE_NAME, CHECKSUM_NAME, PROVENANCE_NAME};
use yolo_release_foundation::digest::bytes_sha256;
use yolo_release_test_support::{
    ARCHIVE_BYTES, REVISION, artifact_audit, error_text, gates, linux_runtimes,
};
use yolo_release_verification::test_support::unseal;
use yolo_release_verification::{VerifyRequest, verify};

struct FakeExecutor {
    artifact_root: PathBuf,
    swap: Option<(PathBuf, PathBuf)>,
    fail: bool,
}

impl CandidateExecuting for FakeExecutor {
    fn execute(&self, _workspace: &Path) -> Result<CandidateExecution> {
        if self.fail {
            bail!("forced candidate failure");
        }
        if let Some((current, moved)) = &self.swap {
            fs::rename(current, moved)?;
            fs::create_dir_all(current)?;
        }
        let artifact = PinnedArtifact::fixture(&self.artifact_root, ARCHIVE_BYTES)?;
        let digest = artifact.sha256()?;
        Ok(CandidateExecution {
            revision: REVISION.to_owned(),
            artifact,
            artifact_audit: artifact_audit(&digest),
            linux_runtimes: linux_runtimes(),
            gates: gates(REVISION),
        })
    }
}

#[test]
fn candidate_creates_and_integrity_verifies_fixed_outputs() {
    let fixture = fixture();
    let executor = executor(&fixture, None, false);
    let provenance = create_with_executor(request(&fixture), &executor).unwrap();
    assert!(!provenance.verification.gate_execution_authenticated);
    assert_eq!(provenance.artifact.sha256, bytes_sha256(ARCHIVE_BYTES));
    assert_eq!(
        provenance.artifact_audit.artifact_sha256,
        provenance.artifact.sha256
    );
    assert!(
        provenance
            .gates
            .iter()
            .all(|gate| gate.source_revision == provenance.source_revision)
    );
    assert_eq!(
        verify(VerifyRequest {
            output_directory: &fixture.output,
        })
        .unwrap(),
        provenance
    );
    for name in [ARCHIVE_NAME, CHECKSUM_NAME, PROVENANCE_NAME] {
        assert!(fixture.output.join(name).is_file());
    }
}

#[test]
fn output_parent_swap_cannot_redirect_publication() {
    let root = tempfile::tempdir().unwrap();
    let current = root.path().join("current");
    let moved = root.path().join("pinned-parent");
    let workspace = root.path().join("workspace");
    let artifact_root = root.path().join("artifact");
    fs::create_dir_all(&current).unwrap();
    fs::create_dir(&workspace).unwrap();
    fs::create_dir(&artifact_root).unwrap();
    let output = current.join("output");
    let executor = FakeExecutor {
        artifact_root,
        swap: Some((current.clone(), moved.clone())),
        fail: false,
    };
    let error = error_text(create_with_executor(
        CandidateRequest {
            workspace: &workspace,
            output_directory: &output,
        },
        &executor,
    ));
    assert!(
        error.contains("retained directory path identity changed"),
        "{error}"
    );
    assert!(!moved.join("output").exists());
    assert!(!current.join("output").join(PROVENANCE_NAME).exists());
}

#[test]
fn failed_pipeline_publishes_no_candidate_claims() {
    let fixture = fixture();
    let executor = executor(&fixture, None, true);
    assert!(create_with_executor(request(&fixture), &executor).is_err());
    assert!(!fixture.output.exists());
}

struct Fixture {
    _root: tempfile::TempDir,
    workspace: PathBuf,
    output: PathBuf,
    artifact_root: PathBuf,
}

impl Drop for Fixture {
    fn drop(&mut self) {
        if self.output.is_dir() {
            unseal(&self.output).unwrap();
        }
    }
}

fn fixture() -> Fixture {
    let root = tempfile::tempdir().unwrap();
    let workspace = root.path().join("workspace");
    let output = root.path().join("output");
    let artifact_root = root.path().join("artifact");
    for directory in [&workspace, &artifact_root] {
        fs::create_dir(directory).unwrap();
    }
    Fixture {
        _root: root,
        workspace,
        output,
        artifact_root,
    }
}

fn request(fixture: &Fixture) -> CandidateRequest<'_> {
    CandidateRequest {
        workspace: &fixture.workspace,
        output_directory: &fixture.output,
    }
}

fn executor(fixture: &Fixture, swap: Option<(PathBuf, PathBuf)>, fail: bool) -> FakeExecutor {
    FakeExecutor {
        artifact_root: fixture.artifact_root.clone(),
        swap,
        fail,
    }
}

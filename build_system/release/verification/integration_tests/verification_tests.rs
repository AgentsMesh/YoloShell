use super::support::{ARCHIVE_BYTES, error_text, provenance};
use super::test_constants::INVALID_AUDIT_INPUT_KIND;
use crate::output::OutputTransaction;
use crate::test_support::{seal, unseal};
use crate::validation::checksum_contents;
use crate::validation_audit::INPUT_ERROR;
use crate::verify::{VerifyRequest, verify};
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use yolo_release_candidate_artifact::PinnedArtifact;
use yolo_release_contracts::{ARCHIVE_NAME, CHECKSUM_NAME, PROVENANCE_NAME, READ_ONLY_FILE_MODE};
use yolo_release_filesystem::constants::USER_WRITE_PERMISSION;
use yolo_release_foundation::digest::bytes_sha256;
use yolo_release_foundation::json::encode;

#[test]
fn integrity_verifier_accepts_coherent_candidate_files() {
    let fixture = Fixture::new();
    let expected = fixture.publish();
    assert_eq!(verify(fixture.request()).unwrap(), expected);
}

#[test]
fn artifact_and_checksum_tampering_are_detected() {
    let fixture = Fixture::new();
    fixture.publish();
    replace(&fixture.output.join(ARCHIVE_NAME), b"tampered");
    assert!(error_text(verify(fixture.request())).contains("artifact digest mismatch"));

    let fixture = Fixture::new();
    fixture.publish();
    replace(&fixture.output.join(CHECKSUM_NAME), b"forged checksum\n");
    assert!(error_text(verify(fixture.request())).contains("checksum file mismatch"));
}

#[test]
fn provenance_cannot_claim_authenticated_gate_execution() {
    let fixture = Fixture::new();
    let mut value = fixture.publish();
    value.verification.gate_execution_authenticated = true;
    replace(
        &fixture.output.join(PROVENANCE_NAME),
        &encode(&value).unwrap(),
    );
    assert!(error_text(verify(fixture.request())).contains("verification scope mismatch"));
}

#[test]
fn standalone_verifier_rejects_false_artifact_audit_binding() {
    let fixture = Fixture::new();
    let mut value = fixture.publish();
    value.artifact_audit.artifact_input = INVALID_AUDIT_INPUT_KIND.to_owned();
    replace(
        &fixture.output.join(PROVENANCE_NAME),
        &encode(&value).unwrap(),
    );
    assert!(error_text(verify(fixture.request())).contains(INPUT_ERROR));
}

struct Fixture {
    _root: tempfile::TempDir,
    output: PathBuf,
    artifact_root: PathBuf,
}

impl Fixture {
    fn new() -> Self {
        let root = tempfile::tempdir().unwrap();
        let output = root.path().join("output");
        let artifact_root = root.path().join("artifact");
        fs::create_dir(&artifact_root).unwrap();
        Self {
            _root: root,
            output,
            artifact_root,
        }
    }

    fn publish(&self) -> yolo_release_evidence_model::ReleaseProvenance {
        let artifact = PinnedArtifact::fixture(&self.artifact_root, ARCHIVE_BYTES).unwrap();
        let digest = bytes_sha256(ARCHIVE_BYTES);
        let value = provenance(&digest, ARCHIVE_BYTES.len() as u64);
        OutputTransaction::prepare(&self.output)
            .unwrap()
            .publish(
                &artifact,
                checksum_contents(&digest).as_bytes(),
                &encode(&value).unwrap(),
            )
            .unwrap();
        value
    }

    fn request(&self) -> VerifyRequest<'_> {
        VerifyRequest {
            output_directory: &self.output,
        }
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        if self.output.is_dir() {
            unseal(&self.output).unwrap();
        }
    }
}

fn replace(path: &Path, bytes: &[u8]) {
    let output = path.parent().unwrap();
    unseal(output).unwrap();
    fs::set_permissions(
        path,
        fs::Permissions::from_mode(READ_ONLY_FILE_MODE | USER_WRITE_PERMISSION),
    )
    .unwrap();
    fs::write(path, bytes).unwrap();
    fs::set_permissions(path, fs::Permissions::from_mode(READ_ONLY_FILE_MODE)).unwrap();
    seal(output).unwrap();
}

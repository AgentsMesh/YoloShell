use super::support::{ARCHIVE_BYTES, OTHER_REVISION, error_text, provenance};
use super::test_constants::{
    BUILD_ID_JSON_KEY, BUNDLE_DESCRIPTOR_JSON_KEY, BUNDLE_ID_JSON_KEY, INVALID_AUDIT_INPUT_KIND,
    SOURCE_REVISION_JSON_KEY,
};
use crate::validation::{checksum_contents, validate_provenance};
use yolo_release_acceptance_receipt::constants::{
    NORMAL_VISIBLE_LAUNCH_CHECK, PENDING_STATUS, REAL_CLAUDE_RECOVERY_CHECK,
    REAL_CODEX_RECOVERY_CHECK, REMOTE_MACOS_RECOVERY_CHECK, SPLIT_CONTROLS_CHECK,
    TERMINAL_INPUT_AND_RENDERING_CHECK, TITLE_BAR_DRAGGING_CHECK,
};
use yolo_release_acceptance_receipt::pending_manual_acceptance;
use yolo_release_contracts::{DOCKER_PRIVATE_EXECUTION_ERROR, SCHEMA_VERSION};
use yolo_release_evidence_model::GateToolEvidence;
use yolo_release_foundation::digest::bytes_sha256;
use yolo_release_foundation::validation::{validate_revision, validate_sha256};

#[path = "validation_boundary_tests.rs"]
mod boundary_tests;

#[test]
fn individually_consistent_commands_cannot_claim_another_snapshot_namespace() {
    let digest = bytes_sha256(ARCHIVE_BYTES);
    let mut value = provenance(&digest, ARCHIVE_BYTES.len() as u64);
    let (requirement, gate) = yolo_release_gate_system::REQUIRED_GATES
        .iter()
        .zip(&mut value.gates)
        .find(|(_, gate)| gate.gate == yolo_release_contracts::LINUX_ARM64_GATE)
        .unwrap();
    gate.execution.namespace_identity.inode += 1;
    gate.commands = requirement
        .command_evidence(&gate.tool, &gate.execution)
        .unwrap();
    assert!(error(&value, &digest).contains("share one execution context"));
}

#[test]
fn exact_candidate_provenance_is_valid() {
    let digest = bytes_sha256(ARCHIVE_BYTES);
    validate_provenance(
        &provenance(&digest, ARCHIVE_BYTES.len() as u64),
        &digest,
        ARCHIVE_BYTES.len() as u64,
    )
    .unwrap();
    assert!(validate_revision(OTHER_REVISION).is_ok());
    assert!(validate_sha256(&digest).is_ok());
    assert!(checksum_contents(&digest).ends_with("  Yolo.zip\n"));
}

#[test]
fn accepted_or_authenticated_claims_are_rejected() {
    let digest = bytes_sha256(ARCHIVE_BYTES);
    let mut value = provenance(&digest, ARCHIVE_BYTES.len() as u64);
    value.verification.gate_execution_authenticated = true;
    assert!(
        error_text(validate_provenance(
            &value,
            &digest,
            ARCHIVE_BYTES.len() as u64
        ))
        .contains("verification scope")
    );
    value = provenance(&digest, ARCHIVE_BYTES.len() as u64);
    value.manual_acceptance.status = "accepted".to_owned();
    assert!(
        error_text(validate_provenance(
            &value,
            &digest,
            ARCHIVE_BYTES.len() as u64
        ))
        .contains("manual acceptance")
    );
    assert_eq!(
        provenance(&digest, ARCHIVE_BYTES.len() as u64)
            .manual_acceptance
            .status,
        PENDING_STATUS
    );
}

#[test]
fn alpha_manual_acceptance_contains_every_release_blocking_workflow() {
    let identifiers = pending_manual_acceptance()
        .checks
        .into_iter()
        .map(|check| check.identifier)
        .collect::<Vec<_>>();
    assert_eq!(
        identifiers,
        [
            NORMAL_VISIBLE_LAUNCH_CHECK,
            TITLE_BAR_DRAGGING_CHECK,
            SPLIT_CONTROLS_CHECK,
            TERMINAL_INPUT_AND_RENDERING_CHECK,
            REAL_CODEX_RECOVERY_CHECK,
            REAL_CLAUDE_RECOVERY_CHECK,
            REMOTE_MACOS_RECOVERY_CHECK,
        ]
    );
}

#[test]
fn forged_gate_shape_and_execution_context_are_rejected() {
    let digest = bytes_sha256(ARCHIVE_BYTES);
    let mut value = provenance(&digest, ARCHIVE_BYTES.len() as u64);
    value.gates[0].commands.clear();
    assert!(error(&value, &digest).contains("command sequence"));
    value = provenance(&digest, ARCHIVE_BYTES.len() as u64);
    value.gates[1].source_revision = OTHER_REVISION.to_owned();
    assert!(error(&value, &digest).contains("source revision"));
    value = provenance(&digest, ARCHIVE_BYTES.len() as u64);
    value.gates[1].execution.isolated_home = "/private/other/home".to_owned();
    assert!(error(&value, &digest).contains("HOME mismatch"));
    value = provenance(&digest, ARCHIVE_BYTES.len() as u64);
    let local = value
        .gates
        .iter_mut()
        .find(|gate| matches!(gate.tool, GateToolEvidence::MacOsBazel { .. }))
        .unwrap();
    if let GateToolEvidence::MacOsBazel { launcher } = &mut local.tool {
        launcher.sha256 = "0".repeat(digest.len());
    }
    assert!(error(&value, &digest).contains("launcher digest"));
    value = provenance(&digest, ARCHIVE_BYTES.len() as u64);
    let linux = value
        .gates
        .iter_mut()
        .find(|gate| matches!(gate.tool, GateToolEvidence::LinuxDocker { .. }))
        .unwrap();
    if let GateToolEvidence::LinuxDocker {
        docker_canonical_path,
        docker_execution_path,
        ..
    } = &mut linux.tool
    {
        *docker_execution_path = docker_canonical_path.clone();
    }
    assert!(error(&value, &digest).contains(DOCKER_PRIVATE_EXECUTION_ERROR));
}

#[test]
fn artifact_audit_and_linux_gates_bind_their_exact_artifacts() {
    let digest = bytes_sha256(ARCHIVE_BYTES);
    let mut value = provenance(&digest, ARCHIVE_BYTES.len() as u64);
    value.artifact_audit.artifact_sha256 = "0".repeat(digest.len());
    assert!(error(&value, &digest).contains("audit digest"));
    value = provenance(&digest, ARCHIVE_BYTES.len() as u64);
    value.artifact_audit.artifact_input = INVALID_AUDIT_INPUT_KIND.to_owned();
    assert!(error(&value, &digest).contains("audit input"));
    let serialized = serde_json::to_value(provenance(&digest, ARCHIVE_BYTES.len() as u64)).unwrap();
    let gate = &serialized["gates"][0];
    assert_eq!(
        gate["tool"][BUNDLE_ID_JSON_KEY],
        serialized["linux_runtimes"][0][BUNDLE_DESCRIPTOR_JSON_KEY][BUILD_ID_JSON_KEY]
    );
    assert_eq!(
        gate[SOURCE_REVISION_JSON_KEY],
        crate::unit_tests::support::REVISION
    );
}

#[test]
fn schema_artifact_and_revision_tampering_are_rejected() {
    let digest = bytes_sha256(ARCHIVE_BYTES);
    let mut value = provenance(&digest, ARCHIVE_BYTES.len() as u64);
    value.schema_version = SCHEMA_VERSION - 1;
    assert!(error(&value, &digest).contains("schema mismatch"));
    value = provenance(&digest, ARCHIVE_BYTES.len() as u64);
    value.source_revision = "not-a-revision".to_owned();
    assert!(error(&value, &digest).contains("Git object ID"));
    value = provenance(&digest, ARCHIVE_BYTES.len() as u64);
    value.artifact.sha256 = "0".repeat(digest.len());
    assert!(error(&value, &digest).contains("artifact digest mismatch"));
}

fn error(value: &yolo_release_evidence_model::ReleaseProvenance, digest: &str) -> String {
    error_text(validate_provenance(
        value,
        digest,
        ARCHIVE_BYTES.len() as u64,
    ))
}

use crate::validation_audit::validate_artifact_audit;
use anyhow::{Context, Result, ensure};
use std::path::Path;
use yolo_release_acceptance_receipt::pending_manual_acceptance;
use yolo_release_contracts::{
    ARCHIVE_NAME, BAZEL_LINK_NAME_PREFIX, CANDIDATE_UNACCEPTED_STATUS, CHECKSUM_LINE_ENDING,
    CHECKSUM_SEPARATOR, HOME_DIRECTORY_NAME, INTEGRITY_ONLY_SCOPE, LINKS_DIRECTORY_NAME,
    LINUX_ARTIFACT_DIRECTORY_NAME, OUTPUT_DIRECTORY_NAME, PRODUCT_NAME, SCHEMA_VERSION,
    SOURCE_DIRECTORY_NAME, TEMP_DIRECTORY_NAME,
};
use yolo_release_evidence_model::{GateExecutionEvidence, ReleaseProvenance};
use yolo_release_foundation::validation::{validate_revision, validate_sha256};
use yolo_release_gate_system::{REQUIRED_GATES, find_linux_runtime, validate_linux_runtimes};

pub fn checksum_contents(digest: &str) -> String {
    format!("{digest}{CHECKSUM_SEPARATOR}{ARCHIVE_NAME}{CHECKSUM_LINE_ENDING}")
}

pub fn validate_provenance(
    provenance: &ReleaseProvenance,
    artifact_sha256: &str,
    artifact_size: u64,
) -> Result<()> {
    ensure!(
        provenance.schema_version == SCHEMA_VERSION,
        "provenance schema mismatch"
    );
    ensure!(provenance.product == PRODUCT_NAME, "product name mismatch");
    validate_revision(&provenance.source_revision)?;
    ensure!(
        provenance.release_status == CANDIDATE_UNACCEPTED_STATUS,
        "release status mismatch"
    );
    ensure!(
        provenance.verification.scope == INTEGRITY_ONLY_SCOPE
            && !provenance.verification.gate_execution_authenticated,
        "verification scope mismatch"
    );
    ensure!(
        provenance.artifact.name == ARCHIVE_NAME,
        "artifact name mismatch"
    );
    validate_sha256(&provenance.artifact.sha256)?;
    ensure!(
        provenance.artifact.sha256 == artifact_sha256,
        "artifact digest mismatch"
    );
    ensure!(
        provenance.artifact.size_bytes == artifact_size,
        "artifact size mismatch"
    );
    validate_artifact_audit(
        &provenance.artifact_audit,
        &provenance.source_revision,
        artifact_sha256,
    )?;
    validate_linux_runtimes(&provenance.linux_runtimes)?;
    ensure!(
        provenance.gates.len() == REQUIRED_GATES.len(),
        "required gate count mismatch"
    );
    for (gate, requirement) in provenance.gates.iter().zip(REQUIRED_GATES) {
        validate_execution(&gate.execution)?;
        crate::validation_gate::validate(
            gate,
            requirement,
            find_linux_runtime(&provenance.linux_runtimes, requirement),
            &provenance.source_revision,
        )?;
    }
    let first = provenance.gates.first().context("missing gate evidence")?;
    ensure!(
        provenance
            .gates
            .iter()
            .all(|gate| { gate.execution == first.execution }),
        "candidate gates did not share one execution context"
    );
    ensure!(
        provenance.manual_acceptance == pending_manual_acceptance(),
        "manual acceptance evidence mismatch"
    );
    Ok(())
}

fn validate_execution(execution: &GateExecutionEvidence) -> Result<()> {
    let workspace = Path::new(&execution.workspace_snapshot);
    ensure!(
        workspace.is_absolute(),
        "gate snapshot path must be absolute"
    );
    let root = workspace
        .parent()
        .context("gate snapshot path has no parent")?;
    ensure!(
        workspace == root.join(SOURCE_DIRECTORY_NAME),
        "gate snapshot layout mismatch"
    );
    ensure!(
        Path::new(&execution.output_user_root) == root.join(OUTPUT_DIRECTORY_NAME),
        "gate output root mismatch"
    );
    ensure!(
        Path::new(&execution.symlink_prefix)
            == root.join(LINKS_DIRECTORY_NAME).join(BAZEL_LINK_NAME_PREFIX),
        "gate symlink prefix mismatch"
    );
    ensure!(
        Path::new(&execution.isolated_home) == root.join(HOME_DIRECTORY_NAME),
        "gate HOME mismatch"
    );
    ensure!(
        Path::new(&execution.isolated_tmp) == root.join(TEMP_DIRECTORY_NAME),
        "gate TMPDIR mismatch"
    );
    ensure!(
        Path::new(&execution.linux_artifact_repository) == root.join(LINUX_ARTIFACT_DIRECTORY_NAME),
        "gate Linux artifact repository mismatch"
    );
    Ok(())
}

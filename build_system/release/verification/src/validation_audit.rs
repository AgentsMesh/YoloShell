use anyhow::{Result, ensure};
use yolo_release_contracts::{
    ARTIFACT_AUDIT_IDENTIFIER, ARTIFACT_AUDIT_INPUT_KIND, AUDIT_SCRIPT_RELATIVE_PATH,
    LINUX_RUNTIME_MANIFEST_INPUT_KIND,
};
use yolo_release_evidence_model::ArtifactAuditEvidence;

const IDENTIFIER_ERROR: &str = "artifact audit identifier mismatch";
const REVISION_ERROR: &str = "artifact audit source revision mismatch";
const DIGEST_ERROR: &str = "artifact audit digest mismatch";
const AUDITOR_ERROR: &str = "artifact auditor source mismatch";
pub(crate) const INPUT_ERROR: &str = "artifact audit input mismatch";

pub(crate) fn validate_artifact_audit(
    audit: &ArtifactAuditEvidence,
    source_revision: &str,
    artifact_sha256: &str,
) -> Result<()> {
    ensure!(audit.audit == ARTIFACT_AUDIT_IDENTIFIER, IDENTIFIER_ERROR);
    ensure!(audit.source_revision == source_revision, REVISION_ERROR);
    ensure!(audit.artifact_sha256 == artifact_sha256, DIGEST_ERROR);
    ensure!(
        audit.auditor_source == AUDIT_SCRIPT_RELATIVE_PATH,
        AUDITOR_ERROR
    );
    ensure!(
        audit.artifact_input == ARTIFACT_AUDIT_INPUT_KIND,
        INPUT_ERROR
    );
    ensure!(
        audit.linux_runtime_manifest_input == LINUX_RUNTIME_MANIFEST_INPUT_KIND,
        "artifact audit Linux manifest input mismatch"
    );
    Ok(())
}

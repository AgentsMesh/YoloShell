pub mod cli_constants;
pub mod constants;

pub use yolo_release_gate_test_support::support::{
    ARCHIVE_BYTES, COMPLETION_SECONDS, EXECUTION_ROOT, OTHER_ARCHIVE_BYTES, OTHER_REVISION,
    REVISION, error_text, execution, fifo, gates, launcher, linux_runtimes,
};

use yolo_release_acceptance_receipt::pending_manual_acceptance;
use yolo_release_contracts::{
    ARCHIVE_NAME, ARTIFACT_AUDIT_IDENTIFIER, ARTIFACT_AUDIT_INPUT_KIND, AUDIT_SCRIPT_RELATIVE_PATH,
    CANDIDATE_UNACCEPTED_STATUS, INTEGRITY_ONLY_SCOPE, PRODUCT_NAME, SCHEMA_VERSION,
};
use yolo_release_evidence_model::{
    ArtifactAuditEvidence, ArtifactEvidence, ReleaseProvenance, VerificationEvidence,
};

pub fn artifact_audit(artifact_sha256: &str) -> ArtifactAuditEvidence {
    ArtifactAuditEvidence {
        audit: ARTIFACT_AUDIT_IDENTIFIER.to_owned(),
        source_revision: REVISION.to_owned(),
        artifact_sha256: artifact_sha256.to_owned(),
        auditor_source: AUDIT_SCRIPT_RELATIVE_PATH.to_owned(),
        artifact_input: ARTIFACT_AUDIT_INPUT_KIND.to_owned(),
        linux_runtime_manifest_input: yolo_release_contracts::LINUX_RUNTIME_MANIFEST_INPUT_KIND
            .to_owned(),
    }
}

pub fn provenance(artifact_sha256: &str, size_bytes: u64) -> ReleaseProvenance {
    ReleaseProvenance {
        schema_version: SCHEMA_VERSION,
        product: PRODUCT_NAME.to_owned(),
        source_revision: REVISION.to_owned(),
        release_status: CANDIDATE_UNACCEPTED_STATUS.to_owned(),
        verification: VerificationEvidence {
            scope: INTEGRITY_ONLY_SCOPE.to_owned(),
            gate_execution_authenticated: false,
        },
        artifact: ArtifactEvidence {
            name: ARCHIVE_NAME.to_owned(),
            sha256: artifact_sha256.to_owned(),
            size_bytes,
        },
        artifact_audit: artifact_audit(artifact_sha256),
        linux_runtimes: linux_runtimes(),
        gates: gates(REVISION),
        manual_acceptance: pending_manual_acceptance(),
    }
}

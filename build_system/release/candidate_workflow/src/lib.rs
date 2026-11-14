use anyhow::{Result, ensure};
use std::path::Path;
use yolo_release_acceptance_receipt::pending_manual_acceptance;
use yolo_release_candidate_artifact::PinnedArtifact;
use yolo_release_contracts::{
    ARCHIVE_NAME, ARTIFACT_CHANGED_ERROR, CANDIDATE_UNACCEPTED_STATUS, INTEGRITY_ONLY_SCOPE,
    PRODUCT_NAME, SCHEMA_VERSION,
};
use yolo_release_evidence_model::{
    ArtifactAuditEvidence, ArtifactEvidence, GateEvidence, ReleaseProvenance, VerificationEvidence,
};
use yolo_release_foundation::json::encode;
use yolo_release_gate_system::{
    GateKind, GateSnapshot, PACKAGE_COMMAND, ProcessExecuting, REQUIRED_GATES,
    SystemProcessExecutor, audit, run_gate,
};
use yolo_release_linux_gate_system::{
    LinuxBazelToolchains, prepare_linux_artifacts, run_linux_gate, seal_linux_artifacts,
};
use yolo_release_tool_custody::DockerClient;
use yolo_release_tool_custody::TrustedBazel;
use yolo_release_verification::output::OutputTransaction;
use yolo_release_verification::validation::checksum_contents;
use yolo_release_verification::verify::verify_opened;
use yolo_release_workspace_state::WorkspaceState;

pub struct CandidateRequest<'a> {
    pub workspace: &'a Path,
    pub output_directory: &'a Path,
}

pub(crate) struct CandidateExecution {
    pub revision: String,
    pub artifact: PinnedArtifact,
    pub artifact_audit: ArtifactAuditEvidence,
    pub linux_runtimes: Vec<yolo_release_evidence_model::LinuxRuntimeEvidence>,
    pub gates: Vec<GateEvidence>,
}

pub(crate) trait CandidateExecuting {
    fn execute(&self, workspace: &Path) -> Result<CandidateExecution>;
}

struct SystemCandidateExecutor;

impl CandidateExecuting for SystemCandidateExecutor {
    fn execute(&self, workspace: &Path) -> Result<CandidateExecution> {
        let state = WorkspaceState::validate(workspace)?;
        let mut snapshot = GateSnapshot::create(&state)?;
        let launcher = TrustedBazel::resolve()?;
        let linux_bazel = LinuxBazelToolchains::resolve()?;
        let docker = DockerClient::resolve()?;
        let executor = SystemProcessExecutor;
        prepare_linux_artifacts(&snapshot)?;
        let mut gates = Vec::new();
        let mut linux_runtimes = Vec::new();
        for requirement in REQUIRED_GATES {
            let GateKind::Linux(profile) = requirement.kind else {
                continue;
            };
            let (gate, runtime) = run_linux_gate(
                requirement,
                state.revision(),
                &snapshot,
                linux_bazel.tool(profile),
                &docker,
                &executor,
            )?;
            gates.push(gate);
            linux_runtimes.push(runtime);
        }
        seal_linux_artifacts(&mut snapshot, state.revision(), &linux_runtimes)?;
        snapshot.verify_integrity()?;
        launcher.ensure_unchanged()?;
        executor.execute(&snapshot.command(&launcher, PACKAGE_COMMAND)?)?;
        launcher.ensure_unchanged()?;
        snapshot.verify_integrity()?;
        let artifact = snapshot.pin_artifact()?;
        let artifact_sha256 = artifact.sha256()?;
        let artifact_audit = audit(&snapshot, &artifact, state.revision(), &linux_runtimes)?;
        for requirement in REQUIRED_GATES {
            if matches!(requirement.kind, GateKind::Bazel(_)) {
                gates.push(run_gate(
                    requirement,
                    state.revision(),
                    &snapshot,
                    &launcher,
                    &executor,
                )?);
            }
        }
        ensure!(
            artifact.sha256()? == artifact_sha256,
            ARTIFACT_CHANGED_ERROR
        );
        launcher.ensure_unchanged()?;
        state.ensure_unchanged()?;
        snapshot.close()?;
        Ok(CandidateExecution {
            revision: state.revision().to_owned(),
            artifact,
            artifact_audit,
            linux_runtimes,
            gates,
        })
    }
}

pub fn create(request: CandidateRequest<'_>) -> Result<ReleaseProvenance> {
    create_with_executor(request, &SystemCandidateExecutor)
}

pub(crate) fn create_with_executor(
    request: CandidateRequest<'_>,
    executor: &dyn CandidateExecuting,
) -> Result<ReleaseProvenance> {
    let output = OutputTransaction::prepare(request.output_directory)?;
    let execution = executor.execute(request.workspace)?;
    let artifact_sha256 = execution.artifact.sha256()?;
    let provenance = ReleaseProvenance {
        schema_version: SCHEMA_VERSION,
        product: PRODUCT_NAME.to_owned(),
        source_revision: execution.revision,
        release_status: CANDIDATE_UNACCEPTED_STATUS.to_owned(),
        verification: VerificationEvidence {
            scope: INTEGRITY_ONLY_SCOPE.to_owned(),
            gate_execution_authenticated: false,
        },
        artifact: ArtifactEvidence {
            name: ARCHIVE_NAME.to_owned(),
            sha256: artifact_sha256.clone(),
            size_bytes: execution.artifact.size_bytes()?,
        },
        artifact_audit: execution.artifact_audit,
        linux_runtimes: execution.linux_runtimes,
        gates: execution.gates,
        manual_acceptance: pending_manual_acceptance(),
    };
    let provenance_bytes = encode(&provenance)?;
    let checksum = checksum_contents(&artifact_sha256);
    let published = output.publish(&execution.artifact, checksum.as_bytes(), &provenance_bytes)?;
    let verified = verify_opened(&published)?;
    ensure!(
        verified == provenance,
        "published candidate verification mismatch"
    );
    Ok(provenance)
}

#[cfg(all(test, feature = "yolo-integration-tests"))]
#[path = "../integration_tests/candidate_tests.rs"]
mod integration_tests;

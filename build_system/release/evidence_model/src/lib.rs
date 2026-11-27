use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct BazelLauncherEvidence {
    pub canonical_path: String,
    pub execution_path: String,
    pub sha256: String,
    pub version: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct DirectoryIdentityEvidence {
    pub device: u64,
    pub inode: u64,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct GateExecutionEvidence {
    pub workspace_snapshot: String,
    pub output_user_root: String,
    pub symlink_prefix: String,
    pub isolated_home: String,
    pub isolated_tmp: String,
    pub linux_artifact_repository: String,
    pub namespace_identity: DirectoryIdentityEvidence,
    pub workspace_identity: DirectoryIdentityEvidence,
    pub output_identity: DirectoryIdentityEvidence,
    pub links_identity: DirectoryIdentityEvidence,
    pub home_identity: DirectoryIdentityEvidence,
    pub tmp_identity: DirectoryIdentityEvidence,
    pub linux_artifact_identity: DirectoryIdentityEvidence,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ArtifactEvidence {
    pub name: String,
    pub sha256: String,
    pub size_bytes: u64,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ArtifactAuditEvidence {
    pub audit: String,
    pub source_revision: String,
    pub artifact_sha256: String,
    pub auditor_source: String,
    pub artifact_input: String,
    pub linux_runtime_manifest_input: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct GateEvidence {
    pub gate: String,
    pub target: String,
    pub source_revision: String,
    pub tool: GateToolEvidence,
    pub execution: GateExecutionEvidence,
    pub commands: Vec<Vec<String>>,
    pub completed_at_unix_seconds: u64,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum GateToolEvidence {
    MacOsBazel {
        launcher: BazelLauncherEvidence,
    },
    LinuxDocker {
        docker_path: String,
        docker_canonical_path: String,
        docker_execution_path: String,
        docker_sha256: String,
        docker_version: String,
        platform: String,
        image: String,
        bazel_path: String,
        bazel_sha256: String,
        bazel_version: String,
        container_user: String,
        bundle_id: String,
    },
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct LinuxRuntimeEvidence {
    pub image: String,
    pub bazel_sha256: String,
    pub bundle_root: String,
    pub descriptor: yolo_distribution_bundle::BundleDescriptor,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct VerificationEvidence {
    pub scope: String,
    pub gate_execution_authenticated: bool,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ManualAcceptanceCheck {
    pub identifier: String,
    pub status: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ManualAcceptanceEvidence {
    pub schema: String,
    pub status: String,
    pub checks: Vec<ManualAcceptanceCheck>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ReleaseProvenance {
    pub schema_version: u16,
    pub product: String,
    pub source_revision: String,
    pub release_status: String,
    pub verification: VerificationEvidence,
    pub artifact: ArtifactEvidence,
    pub artifact_audit: ArtifactAuditEvidence,
    pub linux_runtimes: Vec<LinuxRuntimeEvidence>,
    pub gates: Vec<GateEvidence>,
    pub manual_acceptance: ManualAcceptanceEvidence,
}

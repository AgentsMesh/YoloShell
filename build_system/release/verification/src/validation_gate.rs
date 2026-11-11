use anyhow::{Result, ensure};
use std::path::Path;
use yolo_release_contracts::*;
use yolo_release_evidence_model::{
    BazelLauncherEvidence, GateEvidence, GateToolEvidence, LinuxRuntimeEvidence,
};
use yolo_release_foundation::validation::validate_sha256;
use yolo_release_gate_system::{GateKind, GateRequirement};

pub(crate) fn validate(
    gate: &GateEvidence,
    requirement: GateRequirement,
    runtime: Option<&LinuxRuntimeEvidence>,
    source_revision: &str,
) -> Result<()> {
    ensure!(gate.gate == requirement.name, "gate name mismatch");
    ensure!(gate.target == requirement.target, "gate target mismatch");
    ensure!(
        gate.source_revision == source_revision,
        "gate source revision mismatch"
    );
    match (requirement.kind, &gate.tool) {
        (GateKind::Bazel(_), GateToolEvidence::MacOsBazel { launcher }) => {
            validate_launcher(launcher)?;
            ensure!(runtime.is_none(), "local gate has Linux runtime evidence");
        }
        (
            GateKind::Linux(profile),
            GateToolEvidence::LinuxDocker {
                docker_path,
                docker_canonical_path,
                docker_execution_path,
                docker_sha256,
                docker_version,
                platform,
                image,
                bazel_path,
                bazel_sha256,
                bazel_version,
                container_user,
                bundle_id,
            },
        ) => {
            let runtime =
                runtime.ok_or_else(|| anyhow::anyhow!("missing Linux runtime evidence"))?;
            ensure!(docker_path == DOCKER_SOURCE_LABEL, "Docker source mismatch");
            ensure!(
                Path::new(docker_canonical_path).is_absolute(),
                "Docker canonical path mismatch"
            );
            ensure!(
                Path::new(docker_execution_path).is_absolute()
                    && !docker_execution_path.contains(PATH_DELIMITER),
                DOCKER_EXECUTION_PATH_ERROR
            );
            ensure!(
                docker_execution_path != docker_path
                    && docker_execution_path != docker_canonical_path,
                DOCKER_PRIVATE_EXECUTION_ERROR
            );
            ensure!(
                docker_sha256 == DOCKER_CLIENT_SHA256,
                "Docker client digest mismatch"
            );
            ensure!(
                docker_version == DOCKER_CLIENT_VERSION,
                "Docker client version mismatch"
            );
            ensure!(
                platform == profile.docker_platform,
                "Linux gate platform mismatch"
            );
            ensure!(image == profile.image, "Linux gate image mismatch");
            ensure!(
                bazel_sha256 == profile.bazel_sha256,
                "Linux Bazel digest mismatch"
            );
            ensure!(
                bazel_version == LINUX_BAZEL_VERSION,
                "Linux Bazel version mismatch"
            );
            ensure!(
                Path::new(bazel_path).is_absolute() && !bazel_path.contains(PATH_DELIMITER),
                LINUX_BAZEL_PATH_ERROR
            );
            ensure!(
                valid_user(container_user),
                "Linux container user is invalid"
            );
            validate_sha256(bundle_id)?;
            ensure!(
                bundle_id == runtime.descriptor.build_id(),
                "Linux bundle identity mismatch"
            );
            ensure!(
                runtime.descriptor.source_revision() == source_revision,
                "Linux bundle source revision mismatch"
            );
        }
        _ => anyhow::bail!("release gate tool kind mismatch"),
    }
    ensure!(
        gate.commands == requirement.command_evidence(&gate.tool, &gate.execution)?,
        "gate command sequence mismatch"
    );
    ensure!(
        gate.completed_at_unix_seconds > 0,
        "gate completion time is invalid"
    );
    Ok(())
}

fn validate_launcher(launcher: &BazelLauncherEvidence) -> Result<()> {
    ensure!(
        Path::new(&launcher.canonical_path).is_absolute(),
        "Bazel launcher path must be absolute"
    );
    ensure!(
        Path::new(&launcher.execution_path).is_absolute(),
        "Bazel execution path must be absolute"
    );
    ensure!(
        launcher.sha256 == PINNED_BAZEL_SHA256,
        "Bazel launcher digest mismatch"
    );
    ensure!(
        launcher.version == PINNED_BAZEL_VERSION,
        "Bazel launcher version mismatch"
    );
    Ok(())
}

fn valid_user(value: &str) -> bool {
    let Some((user, group)) = value.split_once(':') else {
        return false;
    };
    !user.is_empty()
        && !group.is_empty()
        && user.bytes().all(|byte| byte.is_ascii_digit())
        && group.bytes().all(|byte| byte.is_ascii_digit())
}

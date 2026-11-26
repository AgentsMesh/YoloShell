use crate::linux_artifact_repository;
use crate::linux_bazel::LinuxBazelTool;
use anyhow::{Result, ensure};
use std::fs;
use std::os::unix::fs::PermissionsExt;
use yolo_release_contracts::*;
use yolo_release_evidence_model::{GateEvidence, GateToolEvidence, LinuxRuntimeEvidence};
use yolo_release_foundation::validation::SHA256_HEX_LENGTH;
use yolo_release_gate_system::{
    GateKind, GateRequirement, GateSnapshot, ProcessExecuting, linux_cgroup_gate_command,
    linux_gate_command,
};
use yolo_release_process_execution::ProcessCommand;
use yolo_release_tool_custody::DockerClientTool;

pub fn run(
    requirement: GateRequirement,
    source_revision: &str,
    snapshot: &GateSnapshot,
    bazel: &dyn LinuxBazelTool,
    docker: &dyn DockerClientTool,
    executor: &dyn ProcessExecuting,
) -> Result<(GateEvidence, LinuxRuntimeEvidence)> {
    let engine = crate::container::engine::SystemEngine { snapshot, docker };
    run_with_engine(
        requirement,
        source_revision,
        snapshot,
        bazel,
        docker,
        executor,
        &engine,
    )
}

pub(crate) fn run_with_engine(
    requirement: GateRequirement,
    source_revision: &str,
    snapshot: &GateSnapshot,
    bazel: &dyn LinuxBazelTool,
    docker: &dyn DockerClientTool,
    executor: &dyn ProcessExecuting,
    engine: &dyn crate::container::engine::Engine,
) -> Result<(GateEvidence, LinuxRuntimeEvidence)> {
    let GateKind::Linux(profile) = requirement.kind else {
        anyhow::bail!("Linux gate runner requires a Linux gate");
    };
    let execution = snapshot.execution()?;
    let output = yolo_release_gate_system::container_mounts::output_path(profile, &execution);
    fs::create_dir(&output)?;
    fs::set_permissions(&output, fs::Permissions::from_mode(PRIVATE_DIRECTORY_MODE))?;
    linux_gate_command::validate_inputs(snapshot.source().display_path(), bazel.path(), &output)?;
    let container_user = format!("{}:{}", unsafe { libc::getuid() }, unsafe {
        libc::getgid()
    });
    let build_arguments =
        linux_gate_command::arguments(profile, &execution, bazel.path(), &container_user)?;
    let cgroup_arguments =
        linux_cgroup_gate_command::arguments(profile, &execution, bazel.path(), &container_user)?;
    let contracts = [
        crate::container::model::Contract::new(
            profile,
            &execution,
            bazel.path(),
            CONTAINER_BUILD_ROLE,
            &build_arguments,
        )?,
        crate::container::model::Contract::new(
            profile,
            &execution,
            bazel.path(),
            CONTAINER_CGROUP_ROLE,
            &cgroup_arguments,
        )?,
    ];
    let commands = [
        ProcessCommand::new(
            docker.path(),
            build_arguments,
            snapshot.source(),
            snapshot.home(),
            snapshot.temporary(),
            "bounded Linux release gate",
        )
        .retain_directories(docker.resource_directory())
        .retain_directories(bazel.resource_directory())
        .with_timeout(LINUX_GATE_TIMEOUT),
        ProcessCommand::new(
            docker.path(),
            cgroup_arguments,
            snapshot.source(),
            snapshot.home(),
            snapshot.temporary(),
            "bounded Linux cgroup kernel gate",
        )
        .retain_directories(docker.resource_directory())
        .retain_directories(bazel.resource_directory())
        .with_timeout(LINUX_CGROUP_GATE_TIMEOUT)
        .with_output_limit(MAX_LINUX_GATE_OUTPUT_BYTES),
    ];
    snapshot.verify_integrity()?;
    bazel.ensure_unchanged()?;
    docker.ensure_unchanged()?;
    for (command, contract) in commands.iter().zip(contracts) {
        let owner = crate::container::Owner::prepare(
            snapshot,
            engine,
            contract,
            bazel.resource_directory(),
        )?;
        owner.finish(executor.execute(command))?;
        docker.ensure_unchanged()?;
        bazel.ensure_unchanged()?;
        snapshot.verify_integrity()?;
    }
    docker.ensure_unchanged()?;
    bazel.ensure_unchanged()?;
    snapshot.verify_integrity()?;
    let runtime = linux_artifact_repository::install(snapshot, profile, &output, source_revision)?;
    ensure!(
        runtime.descriptor.build_id().len() == SHA256_HEX_LENGTH,
        "Linux runtime digest is invalid"
    );
    let tool = GateToolEvidence::LinuxDocker {
        docker_path: DOCKER_SOURCE_LABEL.to_owned(),
        docker_canonical_path: docker.canonical_path().display().to_string(),
        docker_execution_path: docker.path().display().to_string(),
        docker_sha256: docker.sha256().to_owned(),
        docker_version: docker.version().to_owned(),
        platform: profile.docker_platform.to_owned(),
        image: profile.image.to_owned(),
        bazel_path: bazel.path().display().to_string(),
        bazel_sha256: bazel.sha256().to_owned(),
        bazel_version: LINUX_BAZEL_VERSION.to_owned(),
        container_user,
        bundle_id: runtime.descriptor.build_id().to_owned(),
    };
    let commands = commands.iter().map(ProcessCommand::evidence).collect();
    ensure!(
        commands == requirement.command_evidence(&tool, &execution)?,
        "Linux release gate command evidence mismatch"
    );
    let gate = GateEvidence {
        gate: requirement.name.to_owned(),
        target: requirement.target.to_owned(),
        source_revision: source_revision.to_owned(),
        tool,
        execution,
        commands,
        completed_at_unix_seconds: executor.completed_at_unix_seconds()?,
    };
    Ok((gate, runtime))
}

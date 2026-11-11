use crate::container_mounts::ContainerMounts;
use crate::requirements::LinuxGateProfile;
use anyhow::{Result, ensure};
use std::path::Path;
use yolo_release_contracts::*;
use yolo_release_evidence_model::GateExecutionEvidence;

pub(crate) fn evidence(
    profile: LinuxGateProfile,
    execution: &GateExecutionEvidence,
    docker: &Path,
    bazel: &Path,
    container_user: &str,
) -> Result<Vec<String>> {
    ensure!(
        docker.is_absolute() && !path_text(docker)?.contains(PATH_DELIMITER),
        DOCKER_EXECUTION_PATH_ERROR
    );
    Ok(std::iter::once(path_text(docker)?.to_owned())
        .chain(arguments(profile, execution, bazel, container_user)?)
        .collect())
}

pub fn arguments(
    profile: LinuxGateProfile,
    execution: &GateExecutionEvidence,
    bazel: &Path,
    container_user: &str,
) -> Result<Vec<String>> {
    let mounts = ContainerMounts::new(profile, execution, bazel, CONTAINER_BUILD_ROLE)?;
    let mut values = vec![
        DOCKER_RUN_ARGUMENT.to_owned(),
        DOCKER_REMOVE_ARGUMENT.to_owned(),
        DOCKER_PLATFORM_ARGUMENT.to_owned(),
        profile.docker_platform.to_owned(),
        DOCKER_READ_ONLY_ARGUMENT.to_owned(),
        DOCKER_CAP_DROP_ARGUMENT.to_owned(),
        DOCKER_NO_PRIVILEGES_ARGUMENT.to_owned(),
        DOCKER_PIDS_LIMIT_ARGUMENT.to_owned(),
        DOCKER_USER_ARGUMENT.to_owned(),
        container_user.to_owned(),
    ];
    values.extend(
        crate::container_command::ContainerCommand::new(profile, execution, CONTAINER_BUILD_ROLE)?
            .arguments(),
    );
    values.extend(mounts.arguments());
    values.extend([
        DOCKER_TMPFS_ARGUMENT.to_owned(),
        DOCKER_TMPFS.to_owned(),
        DOCKER_WORKDIR_ARGUMENT.to_owned(),
        DOCKER_WORKSPACE.to_owned(),
        DOCKER_ENV_ARGUMENT.to_owned(),
        CONTAINER_HOME_ENVIRONMENT.to_owned(),
        DOCKER_ENV_ARGUMENT.to_owned(),
        CONTAINER_TEMP_ENVIRONMENT.to_owned(),
        profile.image.to_owned(),
        CONTAINER_BASH.to_owned(),
    ]);
    values.extend(
        CONTAINER_BASH_ARGUMENTS
            .iter()
            .map(|value| (*value).to_owned()),
    );
    values.push(script());
    Ok(values)
}

pub fn validate_inputs(workspace: &Path, bazel: &Path, output: &Path) -> Result<()> {
    ensure!(workspace.is_dir(), LINUX_WORKSPACE_PATH_ERROR);
    ensure!(bazel.is_file(), "Linux Bazel input must be a file");
    ensure!(output.is_dir(), LINUX_OUTPUT_PATH_ERROR);
    Ok(())
}

fn path_text(path: &Path) -> Result<&str> {
    path.to_str()
        .ok_or_else(|| anyhow::anyhow!("Linux gate path must be UTF-8"))
}

fn script() -> String {
    let startup = format!(
        "{BAZEL_BATCH_ARGUMENT} {BAZEL_NO_SYSTEM_RC_ARGUMENT} {BAZEL_NO_HOME_RC_ARGUMENT} \
         {BAZEL_NO_WORKSPACE_RC_ARGUMENT} {BAZEL_TRACKED_RC_ARGUMENT} \
         {BAZEL_OUTPUT_ROOT_PREFIX}{DOCKER_OUTPUT_ROOT}"
    );
    let links = format!("{BAZEL_SYMLINK_PREFIX}{DOCKER_SYMLINK_PREFIX}");
    let glibc_version = format!("{GLIBC_IMPLEMENTATION_NAME} {LINUX_MINIMUM_GLIBC_VERSION}");
    let capture =
        crate::linux_runtime_output::capture_target_script(&startup, &links, LINUX_BUILD_TARGET);
    let name = yolo_distribution_bundle::ArtifactRole::Runtime.executable_name();
    let version_output = yolo_version_output();
    let runtime_output = format!(
        "{capture}\n\
        test \"$(\"$runtime_executable\" {BAZEL_VERSION_ARGUMENT})\" = '{version_output}'\n\
        {INSTALL_EXECUTABLE} -m {READ_ONLY_EXECUTABLE_MODE:04o} \"$runtime_executable\" {DOCKER_ARTIFACT_DIRECTORY}/{name}\n\
        /usr/bin/printf '%s\\n' '{version_output}' > {DOCKER_ARTIFACT_DIRECTORY}/{name}{LINUX_BUNDLE_VERSION_SUFFIX}"
    );
    format!(
        "{MAKE_DIRECTORY_COMMAND} {DOCKER_HOME} {DOCKER_TEMPORARY}\n\
         test \"$({GETCONF_EXECUTABLE} {GETCONF_GLIBC_ARGUMENT})\" = '{glibc_version}'\n\
         test \"$({DOCKER_BAZEL} {BAZEL_VERSION_ARGUMENT})\" = '{LINUX_BAZEL_VERSION}'\n\
         {DOCKER_BAZEL} {startup} {BAZEL_BUILD_VERB} {BAZEL_LOCKFILE_ARGUMENT} {links} {BAZEL_OPTIMIZED_ARGUMENT} \
         {SERIAL_JOBS_ARGUMENT} {LINUX_BUILD_TARGET}\n\
         {runtime_output}"
    )
}

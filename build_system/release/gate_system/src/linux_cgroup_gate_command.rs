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
    ensure!(valid_path(docker), DOCKER_EXECUTION_PATH_ERROR);
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
    let mounts = ContainerMounts::new(profile, execution, bazel, CONTAINER_CGROUP_ROLE)?;
    let (user, group) = split_user(container_user)?;
    let mut values = vec![
        DOCKER_RUN_ARGUMENT.to_owned(),
        DOCKER_REMOVE_ARGUMENT.to_owned(),
        DOCKER_PLATFORM_ARGUMENT.to_owned(),
        profile.docker_platform.to_owned(),
        DOCKER_READ_ONLY_ARGUMENT.to_owned(),
        DOCKER_CGROUP_NAMESPACE_ARGUMENT.to_owned(),
        DOCKER_CAP_DROP_ARGUMENT.to_owned(),
        DOCKER_CAP_SYS_ADMIN_ARGUMENT.to_owned(),
        DOCKER_CAP_CHOWN_ARGUMENT.to_owned(),
        DOCKER_CAP_SETUID_ARGUMENT.to_owned(),
        DOCKER_CAP_SETGID_ARGUMENT.to_owned(),
        DOCKER_NO_PRIVILEGES_ARGUMENT.to_owned(),
        DOCKER_PIDS_LIMIT_ARGUMENT.to_owned(),
    ];
    values.extend(
        crate::container_command::ContainerCommand::new(profile, execution, CONTAINER_CGROUP_ROLE)?
            .arguments(),
    );
    values.extend(mounts.arguments());
    values.extend([
        DOCKER_TMPFS_ARGUMENT.to_owned(),
        DOCKER_TMPFS.to_owned(),
        DOCKER_WORKDIR_ARGUMENT.to_owned(),
        DOCKER_WORKSPACE.to_owned(),
        profile.image.to_owned(),
        CONTAINER_BASH.to_owned(),
    ]);
    values.extend(
        CONTAINER_BASH_ARGUMENTS
            .iter()
            .map(|value| (*value).to_owned()),
    );
    values.push(script(user, group));
    Ok(values)
}

fn valid_path(path: &Path) -> bool {
    path.is_absolute()
        && path
            .to_str()
            .is_some_and(|value| !value.contains(PATH_DELIMITER))
}

fn path_text(path: &Path) -> Result<&str> {
    path.to_str()
        .ok_or_else(|| anyhow::anyhow!("Linux cgroup gate path must be UTF-8"))
}

fn split_user(value: &str) -> Result<(&str, &str)> {
    let Some((user, group)) = value.split_once(':') else {
        anyhow::bail!("Linux cgroup gate user must be numeric user:group");
    };
    ensure!(
        user.bytes().all(|byte| byte.is_ascii_digit())
            && group.bytes().all(|byte| byte.is_ascii_digit())
            && user.parse::<u32>().is_ok_and(|id| id != u32::default())
            && group.parse::<u32>().is_ok(),
        "Linux cgroup gate user must be numeric nonroot user:group"
    );
    Ok((user, group))
}

fn script(user: &str, group: &str) -> String {
    let startup = format!(
        "{BAZEL_BATCH_ARGUMENT} {BAZEL_NO_SYSTEM_RC_ARGUMENT} {BAZEL_NO_HOME_RC_ARGUMENT} \
         {BAZEL_NO_WORKSPACE_RC_ARGUMENT} {BAZEL_TRACKED_RC_ARGUMENT} \
         {BAZEL_OUTPUT_ROOT_PREFIX}{DOCKER_OUTPUT_ROOT}"
    );
    let links = format!("{BAZEL_SYMLINK_PREFIX}{DOCKER_SYMLINK_PREFIX}");
    // Migration requires write access to the source and destination's common ancestor.
    // Start tests below the delegated root before dropping setup privileges.
    format!(
        "{MOUNT_EXECUTABLE} {MOUNT_REMOUNT_RW_ARGUMENT} {MOUNT_REMOUNT_RW_VALUE} {CGROUP_FILESYSTEM_ROOT}\n\
         membership=$(< {CGROUP_SELF_MEMBERSHIP_PATH})\n\
         relative=${{membership#{CGROUP_V2_MEMBERSHIP_PREFIX}}}\n\
         test \"$relative\" != \"$membership\"\n\
         test \"$relative\" != '{CGROUP_NAMESPACE_ROOT}'\n\
         test \"$relative\" = \"/${{relative#/}}\"\n\
         test \"${{relative#*..}}\" = \"$relative\"\n\
         test \"${{relative#*//}}\" = \"$relative\"\n\
         container_root={CGROUP_FILESYSTEM_ROOT}$relative\n\
         test -d \"$container_root\"\n\
         test \"$(< \"$container_root/{CGROUP_TYPE_FILE}\")\" = {CGROUP_DOMAIN_TYPE}\n\
         management=$container_root/{CGROUP_MANAGEMENT_NAME}\n\
         delegated=$container_root/{CGROUP_DELEGATED_NAME}\n\
         /bin/mkdir \"$management\"\n\
         while IFS= read -r member; do test -n \"$member\"; printf '%s\\n' \"$member\" > \"$management/{CGROUP_PROCESSES_FILE}\"; done < \"$container_root/{CGROUP_PROCESSES_FILE}\"\n\
         test -z \"$(< \"$container_root/{CGROUP_PROCESSES_FILE}\")\"\n\
         printf '{CGROUP_PIDS_ENABLE}\\n' > \"$container_root/{CGROUP_SUBTREE_CONTROL_FILE}\"\n\
         /bin/mkdir \"$delegated\"\n\
         test \"$(< \"$delegated/{CGROUP_TYPE_FILE}\")\" = {CGROUP_DOMAIN_TYPE}\n\
         printf '{CGROUP_PIDS_ENABLE}\\n' > \"$delegated/{CGROUP_SUBTREE_CONTROL_FILE}\"\n\
         runner=$delegated/{CGROUP_RUNNER_NAME}\n\
         /bin/mkdir \"$runner\"\n\
         {CHOWN_EXECUTABLE} {user}:{group} \"$delegated\" \"$delegated/{CGROUP_PROCESSES_FILE}\" \"$delegated/{CGROUP_SUBTREE_CONTROL_FILE}\" \"$delegated/{CGROUP_THREADS_FILE}\"\n\
         printf '%s\\n' \"$$\" > \"$runner/{CGROUP_PROCESSES_FILE}\"\n\
         {INSTALL_EXECUTABLE} -d -m 0700 -o {user} -g {group} {DOCKER_HOME} {DOCKER_TEMPORARY} {DOCKER_LINKS_DIRECTORY}\n\
         exec {SETPRIV_EXECUTABLE} {SETPRIV_REUID_PREFIX}{user} {SETPRIV_REGID_PREFIX}{group} {SETPRIV_CLEAR_GROUPS_ARGUMENT} \
         {ENV_EXECUTABLE} {CONTAINER_HOME_ENVIRONMENT} {CONTAINER_TEMP_ENVIRONMENT} {CGROUP_DELEGATED_ENVIRONMENT}=\"$delegated\" \
         {DOCKER_BAZEL} {startup} {BAZEL_TEST_VERB} {BAZEL_LOCKFILE_ARGUMENT} {links} \
         {BAZEL_OPTIMIZED_ARGUMENT} {SERIAL_JOBS_ARGUMENT} {NO_CACHE_ARGUMENT} \
         {BAZEL_TEST_OUTPUT_ARGUMENT} {LINUX_TEST_STRATEGY_ARGUMENT} {LINUX_TEST_SPAWN_STRATEGY_ARGUMENT} \
         {BAZEL_TEST_ENVIRONMENT_PREFIX}{CGROUP_DELEGATED_ENVIRONMENT}=\"$delegated\" \
         {LINUX_TEST_TARGET} {LINUX_CGROUP_TEST_TARGET}"
    )
}

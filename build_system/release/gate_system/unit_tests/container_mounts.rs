use super::support::execution;
use crate::{
    LINUX_AMD64_PROFILE, LINUX_ARM64_PROFILE, container_mounts::ContainerMounts,
    linux_cgroup_gate_command, linux_gate_command,
};
use std::os::unix::ffi::OsStringExt;
use std::path::{Path, PathBuf};
use yolo_release_contracts::*;

const BAZEL_PATH: &str = "/fixture/linux-bazel";
const CONTAINER_USER: &str = "501:20";
const UNKNOWN_ROLE: &str = "unknown-role";
const INVALID_PATHS: &[&str] = &["relative-path", "/fixture,readonly=false"];
const ARGUMENT_PAIR_LENGTH: usize = 2;
const ARGUMENT_FLAG: usize = 0;
const ARGUMENT_VALUE: usize = 1;
const INVALID_UTF8_PATH: &[u8] = b"/fixture/\xff";

#[test]
fn both_commands_render_only_the_fixed_bind_policy_for_each_architecture() {
    let execution = execution();
    let bazel = Path::new(BAZEL_PATH);
    for profile in [LINUX_AMD64_PROFILE, LINUX_ARM64_PROFILE] {
        for role in [CONTAINER_BUILD_ROLE, CONTAINER_CGROUP_ROLE] {
            let arguments = if role == CONTAINER_BUILD_ROLE {
                linux_gate_command::arguments(profile, &execution, bazel, CONTAINER_USER)
            } else {
                linux_cgroup_gate_command::arguments(profile, &execution, bazel, CONTAINER_USER)
            }
            .unwrap();
            let actual = arguments
                .windows(ARGUMENT_PAIR_LENGTH)
                .filter(|pair| pair[ARGUMENT_FLAG] == DOCKER_MOUNT_ARGUMENT)
                .map(|pair| pair[ARGUMENT_VALUE].as_str())
                .collect::<Vec<_>>();
            let mut expected = vec![
                format!(
                    "type=bind,source={},target=/workspace,readonly",
                    execution.workspace_snapshot
                ),
                format!("type=bind,source={BAZEL_PATH},target=/yolo-tools/bazel,readonly"),
            ];
            if role == CONTAINER_BUILD_ROLE {
                expected.push(format!(
                    "type=bind,source={}/{},target=/yolo-artifacts",
                    execution.isolated_tmp, profile.docker_architecture
                ));
            }
            assert_eq!(actual, expected);
        }
    }
}

#[test]
fn only_known_roles_and_safe_absolute_sources_can_form_a_trusted_specification() {
    let profile = LINUX_AMD64_PROFILE;
    let mut execution = execution();
    let bazel = Path::new(BAZEL_PATH);
    assert!(ContainerMounts::new(profile, &execution, bazel, UNKNOWN_ROLE).is_err());
    for invalid in INVALID_PATHS {
        for role in [CONTAINER_BUILD_ROLE, CONTAINER_CGROUP_ROLE] {
            assert!(ContainerMounts::new(profile, &execution, Path::new(invalid), role).is_err());
            let original = std::mem::replace(&mut execution.workspace_snapshot, (*invalid).into());
            assert!(ContainerMounts::new(profile, &execution, bazel, role).is_err());
            execution.workspace_snapshot = original;
        }
        let original = std::mem::replace(&mut execution.isolated_tmp, (*invalid).into());
        assert!(ContainerMounts::new(profile, &execution, bazel, CONTAINER_BUILD_ROLE).is_err());
        assert!(ContainerMounts::new(profile, &execution, bazel, CONTAINER_CGROUP_ROLE).is_ok());
        execution.isolated_tmp = original;
    }
    let invalid = PathBuf::from(std::ffi::OsString::from_vec(INVALID_UTF8_PATH.into()));
    assert!(ContainerMounts::new(profile, &execution, &invalid, CONTAINER_BUILD_ROLE).is_err());
}

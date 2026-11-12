use super::support::execution;
use crate::linux_cgroup_gate_command::arguments;
use crate::requirements::LINUX_ARM64_PROFILE;
use std::path::Path;
use yolo_release_contracts::*;

const PINNED_LINUX_BAZEL: &str = "/private/pinned/linux-bazel";
const CONTAINER_USER: &str = "501:20";
const MALICIOUS_USER: &str = "501:20;id";
const BAZEL_TARGET_PREFIX: &str = "//";
const INVALID_USERS: &[&str] = &[
    "501",
    "0:20",
    "000:20",
    ":20",
    "501:",
    "+501:20",
    "501:+20",
    "4294967296:20",
    "501:4294967296",
    MALICIOUS_USER,
];

fn command() -> Vec<String> {
    arguments(
        LINUX_ARM64_PROFILE,
        &execution(),
        Path::new(PINNED_LINUX_BAZEL),
        CONTAINER_USER,
    )
    .unwrap()
}

#[test]
fn kernel_gate_uses_real_host_cgroup_namespace_with_bounded_setup_capabilities() {
    let values = command();
    assert!(values.contains(&DOCKER_CGROUP_NAMESPACE_ARGUMENT.to_owned()));
    assert!(values.contains(&DOCKER_CAP_DROP_ARGUMENT.to_owned()));
    assert!(values.contains(&DOCKER_CAP_SYS_ADMIN_ARGUMENT.to_owned()));
    assert!(values.contains(&DOCKER_CAP_CHOWN_ARGUMENT.to_owned()));
    assert!(values.contains(&DOCKER_CAP_SETUID_ARGUMENT.to_owned()));
    assert!(values.contains(&DOCKER_CAP_SETGID_ARGUMENT.to_owned()));
    assert!(values.contains(&DOCKER_NO_PRIVILEGES_ARGUMENT.to_owned()));
    assert!(!values.contains(&DOCKER_PRIVILEGED_ARGUMENT.to_owned()));
    assert!(!values.contains(&DOCKER_USER_ARGUMENT.to_owned()));
}

#[test]
fn runtime_gate_mounts_only_pinned_inputs_and_drops_to_the_release_user() {
    let values = command();
    assert!(
        values
            .iter()
            .any(|value| value.contains("target=/workspace,readonly"))
    );
    assert!(
        values
            .iter()
            .any(|value| value.contains("target=/yolo-tools/bazel,readonly"))
    );
    let script = values.last().unwrap();
    assert!(script.contains(MOUNT_REMOUNT_RW_VALUE));
    assert!(script.contains(CGROUP_SELF_MEMBERSHIP_PATH));
    assert!(script.contains(CGROUP_MANAGEMENT_NAME));
    assert!(script.contains(CGROUP_DELEGATED_NAME));
    assert!(script.contains(CGROUP_RUNNER_NAME));
    assert!(script.contains(SETPRIV_EXECUTABLE));
    assert!(script.contains(&format!("{SETPRIV_REUID_PREFIX}501")));
    assert!(script.contains(&format!("{SETPRIV_REGID_PREFIX}20")));
    assert!(script.contains(CGROUP_DELEGATED_ENVIRONMENT));
    assert!(script.contains(LINUX_CGROUP_TEST_TARGET));
}

#[test]
fn kernel_gate_rejects_user_and_path_injection() {
    let context = execution();
    let bazel = Path::new(PINNED_LINUX_BAZEL);
    for user in INVALID_USERS {
        assert!(arguments(LINUX_ARM64_PROFILE, &context, bazel, user).is_err());
    }
    let mut injected = context;
    injected.workspace_snapshot = "/private/source,injected".to_owned();
    assert!(arguments(LINUX_ARM64_PROFILE, &injected, bazel, CONTAINER_USER).is_err());
}

#[test]
fn full_runtime_and_kernel_tests_share_explicit_unsandboxed_delegation() {
    let values = command();
    let script = values.last().unwrap();
    let commands: Vec<_> = script
        .lines()
        .map(|line| line.split_whitespace().collect::<Vec<_>>())
        .filter(|values| values.contains(&DOCKER_BAZEL) && values.contains(&BAZEL_TEST_VERB))
        .collect();
    let [arguments] = commands.as_slice() else {
        panic!("delegated gate must invoke exactly one Bazel test command");
    };
    assert_eq!(
        arguments
            .iter()
            .copied()
            .filter(|value| value.starts_with(BAZEL_JOBS_ARGUMENT_PREFIX))
            .collect::<Vec<_>>(),
        [SERIAL_JOBS_ARGUMENT]
    );
    assert!(arguments.contains(&LINUX_TEST_STRATEGY_ARGUMENT));
    assert!(arguments.contains(&LINUX_TEST_SPAWN_STRATEGY_ARGUMENT));
    assert!(arguments.contains(&NO_CACHE_ARGUMENT));
    assert!(arguments.contains(&BAZEL_OPTIMIZED_ARGUMENT));
    assert_eq!(
        arguments
            .iter()
            .copied()
            .filter(|value| value.starts_with(BAZEL_TEST_ENVIRONMENT_PREFIX))
            .collect::<Vec<_>>(),
        [format!(
            "{BAZEL_TEST_ENVIRONMENT_PREFIX}{CGROUP_DELEGATED_ENVIRONMENT}=\"$delegated\""
        )]
    );
    assert_eq!(
        arguments
            .iter()
            .copied()
            .filter(|value| value.starts_with(BAZEL_TARGET_PREFIX))
            .collect::<Vec<_>>(),
        [LINUX_TEST_TARGET, LINUX_CGROUP_TEST_TARGET]
    );
}

#[test]
fn runner_is_inside_delegation_before_privilege_drop_and_process_migration() {
    let values = command();
    let script = values.last().unwrap();
    let steps = [
        format!("test \"$relative\" != '{CGROUP_NAMESPACE_ROOT}'"),
        format!("management=$container_root/{CGROUP_MANAGEMENT_NAME}"),
        "/bin/mkdir \"$management\"".to_owned(),
        format!("test -z \"$(< \"$container_root/{CGROUP_PROCESSES_FILE}\")\""),
        format!(
            "printf '{CGROUP_PIDS_ENABLE}\\n' > \"$container_root/{CGROUP_SUBTREE_CONTROL_FILE}\""
        ),
        "/bin/mkdir \"$delegated\"".to_owned(),
        format!("printf '{CGROUP_PIDS_ENABLE}\\n' > \"$delegated/{CGROUP_SUBTREE_CONTROL_FILE}\""),
        format!("runner=$delegated/{CGROUP_RUNNER_NAME}"),
        "/bin/mkdir \"$runner\"".to_owned(),
        format!("{CHOWN_EXECUTABLE} {CONTAINER_USER} \"$delegated\""),
        format!("printf '%s\\n' \"$$\" > \"$runner/{CGROUP_PROCESSES_FILE}\""),
        format!("exec {SETPRIV_EXECUTABLE}"),
    ];
    let mut remainder = script.as_str();
    for step in steps {
        let Some((_, next)) = remainder.split_once(&step) else {
            panic!("delegation step is missing or reordered: {step}");
        };
        remainder = next;
    }
}

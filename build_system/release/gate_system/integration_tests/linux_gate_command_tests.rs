use super::support::execution;
use crate::container_mounts::output_path;
use crate::linux_gate_command::{arguments, validate_inputs};
use crate::requirements::{LINUX_AMD64_PROFILE, LINUX_ARM64_PROFILE};
use std::fs;
use yolo_release_contracts::*;
use yolo_release_foundation::validation::SHA256_HEX_LENGTH;

const OCI_DIGEST_MARKER: &str = "@sha256:";
const PINNED_LINUX_BAZEL: &str = "/private/pinned/linux-bazel";
const CONTAINER_USER: &str = "501:20";
const SINGLE_RUNTIME_QUERY: usize = 1;
const RETIRED_RUNTIME_OUTPUT: &str = "bin/core/yolo-host";
const BAZEL_TARGET_PREFIX: &str = "//";

fn linux_script() -> String {
    let execution = execution();
    let bazel = std::path::Path::new(PINNED_LINUX_BAZEL);
    arguments(LINUX_AMD64_PROFILE, &execution, bazel, CONTAINER_USER)
        .unwrap()
        .pop()
        .unwrap()
}

fn bazel_command_arguments<'a>(script: &'a str, verb: &str) -> Vec<&'a str> {
    let mut commands = script
        .lines()
        .map(|line| line.split_whitespace().collect::<Vec<_>>())
        .filter(|values| values.first() == Some(&DOCKER_BAZEL) && values.contains(&verb));
    let command = commands.next().unwrap();
    assert!(commands.next().is_none());
    command
}

fn assert_exact_serial_jobs(arguments: &[&str]) {
    let mut jobs = arguments
        .iter()
        .copied()
        .filter(|value| value.starts_with(BAZEL_JOBS_ARGUMENT_PREFIX));
    assert_eq!(jobs.next(), Some(SERIAL_JOBS_ARGUMENT));
    assert_eq!(jobs.next(), None);
}

#[test]
fn command_builds_the_optimized_runtime_without_attempting_uncontained_tests() {
    let execution = execution();
    let bazel = std::path::Path::new(PINNED_LINUX_BAZEL);
    let values = arguments(LINUX_AMD64_PROFILE, &execution, bazel, CONTAINER_USER).unwrap();
    assert_eq!(values[0], DOCKER_RUN_ARGUMENT);
    assert!(values.contains(&LINUX_AMD64_IMAGE.to_owned()));
    assert!(values.contains(&DOCKER_READ_ONLY_ARGUMENT.to_owned()));
    assert!(values.iter().any(|value| {
        value.contains("target=/workspace,readonly")
            && value.contains(&execution.workspace_snapshot)
    }));
    assert!(values.iter().any(|value| {
        value.contains("target=/yolo-tools/bazel,readonly")
            && value.contains(bazel.to_str().unwrap())
    }));
    let script = values.last().unwrap();
    assert!(script.contains(BAZEL_OPTIMIZED_ARGUMENT));
    assert!(!script.contains(LINUX_TEST_TARGET));
    assert!(!script.contains(LINUX_CGROUP_TEST_TARGET));
    assert!(!script.contains(CGROUP_DELEGATED_ENVIRONMENT));
    assert!(!values.contains(&DOCKER_CAP_SYS_ADMIN_ARGUMENT.to_owned()));
    assert!(values.contains(&DOCKER_USER_ARGUMENT.to_owned()));
    assert!(script.contains(LINUX_BUILD_TARGET));
    for role in yolo_distribution_bundle::ArtifactRole::ALL {
        assert!(script.contains(&format!(
            "{} {LINUX_RUNTIME_VERSION}",
            role.executable_name()
        )));
    }
    assert!(script.contains(BAZEL_LOCKFILE_ARGUMENT));
}

#[test]
fn linux_build_command_has_exactly_one_serial_jobs_argument() {
    let script = linux_script();
    let command = bazel_command_arguments(&script, BAZEL_BUILD_VERB);
    assert_exact_serial_jobs(&command);
    assert_eq!(
        command
            .into_iter()
            .filter(|value| value.starts_with(BAZEL_TARGET_PREFIX))
            .collect::<Vec<_>>(),
        [LINUX_BUILD_TARGET]
    );
}

#[test]
fn build_images_are_distinct_exact_oci_manifest_pins() {
    let images = [LINUX_AMD64_PROFILE.image, LINUX_ARM64_PROFILE.image];
    assert_ne!(images[0], images[1]);
    for image in images {
        let (_, digest) = image.split_once(OCI_DIGEST_MARKER).unwrap();
        assert_eq!(digest.len(), SHA256_HEX_LENGTH);
        assert!(
            digest
                .bytes()
                .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
        );
        assert_eq!(image.matches(OCI_DIGEST_MARKER).count(), 1);
    }
}

#[test]
fn command_proves_the_container_baseline_before_build_and_runs_the_final_elf() {
    let execution = execution();
    let bazel = std::path::Path::new(PINNED_LINUX_BAZEL);
    let values = arguments(LINUX_AMD64_PROFILE, &execution, bazel, CONTAINER_USER).unwrap();
    let script = values.last().unwrap();
    let baseline = format!(
        "test \"$({GETCONF_EXECUTABLE} {GETCONF_GLIBC_ARGUMENT})\" = \
         '{GLIBC_IMPLEMENTATION_NAME} {LINUX_MINIMUM_GLIBC_VERSION}'"
    );
    let runtime_probe = format!("\"$runtime_executable\" {BAZEL_VERSION_ARGUMENT}");
    let baseline_position = script.find(&baseline).unwrap();
    let build_position = script.find(BAZEL_BUILD_VERB).unwrap();
    let runtime_position = script.find(&runtime_probe).unwrap();
    let query_position = script.find(BAZEL_CQUERY_VERB).unwrap();
    assert!(baseline_position < build_position);
    assert!(build_position < query_position);
    assert!(query_position < runtime_position);
    assert!(script[runtime_position..].contains(&format!(
        "{INSTALL_EXECUTABLE} -m {READ_ONLY_EXECUTABLE_MODE:04o} \"$runtime_executable\""
    )));
}

#[test]
fn executable_query_uses_the_built_alias_and_identical_configuration() {
    let script = linux_script();
    let build = bazel_command_arguments(&script, BAZEL_BUILD_VERB);
    let queries = script
        .lines()
        .filter(|line| line.contains(BAZEL_CQUERY_VERB))
        .collect::<Vec<_>>();
    assert_eq!(queries.len(), SINGLE_RUNTIME_QUERY);
    for query in queries {
        assert!(query.contains(LINUX_BUILD_TARGET));
        for argument in build
            .iter()
            .skip(1)
            .filter(|argument| **argument != BAZEL_BUILD_VERB && **argument != LINUX_BUILD_TARGET)
        {
            assert!(
                query
                    .split_whitespace()
                    .any(|value| value.trim_end_matches(')') == *argument)
            );
        }
        assert!(query.contains(BAZEL_QUERY_STARLARK_OUTPUT_ARGUMENT));
        assert!(query.contains(BAZEL_QUERY_EXECUTABLE_EXPRESSION_ARGUMENT));
    }
    assert!(!script.contains(RETIRED_RUNTIME_OUTPUT));
}

#[test]
fn mount_paths_reject_comma_injection_and_non_directories() {
    let root = tempfile::tempdir().unwrap();
    let workspace = root.path().join("workspace");
    let output = root.path().join("output");
    let bazel = root.path().join("bazel");
    fs::create_dir(&workspace).unwrap();
    fs::create_dir(&output).unwrap();
    fs::write(&bazel, b"bazel").unwrap();
    assert!(validate_inputs(&workspace, &bazel, &output).is_ok());
    assert!(validate_inputs(&bazel, &bazel, &output).is_err());
    let mut context = execution();
    context.workspace_snapshot = "/private/source,injected".to_owned();
    assert!(arguments(LINUX_AMD64_PROFILE, &context, &bazel, CONTAINER_USER).is_err());
}

#[test]
fn architecture_output_is_inside_the_producer_temporary_directory() {
    let execution = execution();
    assert_eq!(
        output_path(LINUX_AMD64_PROFILE, &execution),
        std::path::Path::new(&execution.isolated_tmp).join(LINUX_AMD64_DOCKER_ARCHITECTURE)
    );
}

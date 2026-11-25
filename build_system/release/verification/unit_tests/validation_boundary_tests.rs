use super::*;
use yolo_release_evidence_model::{
    BazelLauncherEvidence, GateExecutionEvidence, ReleaseProvenance,
};

fn fresh() -> (ReleaseProvenance, String) {
    let digest = bytes_sha256(ARCHIVE_BYTES);
    (provenance(&digest, ARCHIVE_BYTES.len() as u64), digest)
}

#[test]
fn every_top_level_claim_remains_bound_to_the_candidate() {
    for (change, expected) in [
        (
            (|value: &mut ReleaseProvenance| value.product.push('!')) as fn(&mut ReleaseProvenance),
            "product name",
        ),
        (
            |value: &mut ReleaseProvenance| value.release_status.push('!'),
            "release status",
        ),
        (
            |value: &mut ReleaseProvenance| value.verification.scope.push('!'),
            "verification scope",
        ),
        (
            |value: &mut ReleaseProvenance| value.artifact.name.push('!'),
            "artifact name",
        ),
        (
            |value: &mut ReleaseProvenance| value.artifact.size_bytes += 1,
            "artifact size",
        ),
        (
            |value: &mut ReleaseProvenance| {
                value.gates.pop();
            },
            "gate count",
        ),
        (
            |value: &mut ReleaseProvenance| value.gates[0].gate.push('!'),
            "gate name",
        ),
        (
            |value: &mut ReleaseProvenance| value.gates[0].target.push('!'),
            "gate target",
        ),
        (
            |value: &mut ReleaseProvenance| value.gates[0].completed_at_unix_seconds = 0,
            "completion time",
        ),
    ] {
        let (mut value, digest) = fresh();
        change(&mut value);
        assert!(error(&value, &digest).contains(expected), "{expected}");
    }
}

#[test]
fn every_execution_directory_is_bound_to_the_same_private_snapshot_root() {
    for (change, expected) in [
        (
            (|value: &mut GateExecutionEvidence| value.workspace_snapshot.clear())
                as fn(&mut GateExecutionEvidence),
            "path must be absolute",
        ),
        (
            |value: &mut GateExecutionEvidence| value.workspace_snapshot = "/".into(),
            "has no parent",
        ),
        (
            |value: &mut GateExecutionEvidence| value.workspace_snapshot.push_str("-other"),
            "snapshot layout",
        ),
        (
            |value: &mut GateExecutionEvidence| value.output_user_root.push('!'),
            "output root",
        ),
        (
            |value: &mut GateExecutionEvidence| value.symlink_prefix.push('!'),
            "symlink prefix",
        ),
        (
            |value: &mut GateExecutionEvidence| value.isolated_home.push('!'),
            "HOME mismatch",
        ),
        (
            |value: &mut GateExecutionEvidence| value.isolated_tmp.push('!'),
            "TMPDIR mismatch",
        ),
        (
            |value: &mut GateExecutionEvidence| value.linux_artifact_repository.push('!'),
            "artifact repository",
        ),
    ] {
        let (mut value, digest) = fresh();
        change(&mut value.gates[0].execution);
        assert!(error(&value, &digest).contains(expected), "{expected}");
    }
}

#[test]
fn local_launchers_require_absolute_paths_and_the_pinned_version() {
    for (change, expected) in [
        (
            (|value: &mut BazelLauncherEvidence| value.canonical_path = "relative".into())
                as fn(&mut BazelLauncherEvidence),
            "launcher path",
        ),
        (
            |value: &mut BazelLauncherEvidence| value.execution_path = "relative".into(),
            "execution path",
        ),
        (
            |value: &mut BazelLauncherEvidence| value.version.push('!'),
            "launcher version",
        ),
    ] {
        let (mut value, digest) = fresh();
        let gate = value
            .gates
            .iter_mut()
            .find(|gate| matches!(gate.tool, GateToolEvidence::MacOsBazel { .. }))
            .unwrap();
        let GateToolEvidence::MacOsBazel { launcher } = &mut gate.tool else {
            unreachable!()
        };
        change(launcher);
        assert!(error(&value, &digest).contains(expected), "{expected}");
    }
}

#[test]
fn linux_tool_fields_cannot_substitute_another_runtime_or_execution_source() {
    let separator = yolo_release_contracts::PATH_DELIMITER;
    macro_rules! changed {
        ($field:ident, $replacement:expr, $expected:expr) => {{
            let (mut value, digest) = fresh();
            let gate = value
                .gates
                .iter_mut()
                .find(|gate| matches!(gate.tool, GateToolEvidence::LinuxDocker { .. }))
                .unwrap();
            let GateToolEvidence::LinuxDocker { $field, .. } = &mut gate.tool else {
                unreachable!()
            };
            *$field = $replacement.to_owned();
            let actual = error(&value, &digest);
            assert!(
                actual.contains($expected),
                "{}: {actual}",
                stringify!($field)
            );
        }};
    }
    changed!(docker_path, "other", "Docker source");
    changed!(docker_canonical_path, "relative", "canonical path");
    changed!(
        docker_execution_path,
        "relative",
        yolo_release_contracts::DOCKER_EXECUTION_PATH_ERROR
    );
    changed!(
        docker_execution_path,
        format!("/private/invalid{separator}docker"),
        yolo_release_contracts::DOCKER_EXECUTION_PATH_ERROR
    );
    changed!(docker_sha256, "invalid", "Docker client digest");
    changed!(docker_version, "other", "Docker client version");
    changed!(platform, "other", "gate platform");
    changed!(image, "other", "gate image");
    changed!(bazel_sha256, "invalid", "Linux Bazel digest");
    changed!(bazel_version, "other", "Linux Bazel version");
    changed!(
        bazel_path,
        "relative",
        yolo_release_contracts::LINUX_BAZEL_PATH_ERROR
    );
    changed!(
        bazel_path,
        format!("/private/invalid{separator}bazel"),
        yolo_release_contracts::LINUX_BAZEL_PATH_ERROR
    );
    for user in ["", "1", ":1", "1:", "user:1", "1:group", "1:2:3"] {
        changed!(container_user, user, "container user");
    }
    changed!(bundle_id, "invalid", "SHA-256");
    changed!(
        bundle_id,
        bytes_sha256(b"another archive"),
        "Linux bundle identity"
    );
}

use crate::bundles;
use crate::constants::{
    LINUX_BAZEL_EXECUTION_TEST_PATH_PREFIX, TRUSTED_BAZEL_CANONICAL_TEST_PATH,
    TRUSTED_BAZEL_EXECUTION_TEST_PATH,
};
use std::ffi::CString;
use std::os::unix::ffi::OsStrExt;
use std::path::Path;
use yolo_release_contracts::{PINNED_BAZEL_SHA256, PINNED_BAZEL_VERSION, PRIVATE_FILE_MODE};
use yolo_release_evidence_model::{
    BazelLauncherEvidence, DirectoryIdentityEvidence, GateEvidence, GateExecutionEvidence,
    GateToolEvidence, LinuxRuntimeEvidence,
};
use yolo_release_gate_system::{GateKind, REQUIRED_GATES};

pub const ARCHIVE_BYTES: &[u8] = b"stable candidate archive\n";
pub const OTHER_ARCHIVE_BYTES: &[u8] = b"different candidate archive\n";
pub const REVISION: &str = "0123456789abcdef0123456789abcdef01234567";
pub const OTHER_REVISION: &str = "1123456789abcdef0123456789abcdef01234567";
pub const COMPLETION_SECONDS: u64 = 1_775_000_001;
pub const EXECUTION_ROOT: &str = "/private/tmp/yolo-release-test";
const DOCKER_CANONICAL_TEST_PATH: &str = "/Applications/ContainerRuntime/docker";
const DOCKER_EXECUTION_TEST_PATH: &str = "/private/pinned/docker";
const LINUX_CONTAINER_TEST_USER: &str = "501:20";
const TEST_DEVICE_ID: u64 = 42;
const TEST_INODE_ID: u64 = 84;
const SYSTEM_CALL_SUCCESS: libc::c_int = 0;

pub fn launcher() -> BazelLauncherEvidence {
    BazelLauncherEvidence {
        canonical_path: TRUSTED_BAZEL_CANONICAL_TEST_PATH.to_owned(),
        execution_path: TRUSTED_BAZEL_EXECUTION_TEST_PATH.to_owned(),
        sha256: PINNED_BAZEL_SHA256.to_owned(),
        version: PINNED_BAZEL_VERSION.to_owned(),
    }
}

pub fn execution() -> GateExecutionEvidence {
    let identity = DirectoryIdentityEvidence {
        device: TEST_DEVICE_ID,
        inode: TEST_INODE_ID,
    };
    GateExecutionEvidence {
        workspace_snapshot: format!("{EXECUTION_ROOT}/source"),
        output_user_root: format!("{EXECUTION_ROOT}/output"),
        symlink_prefix: format!("{EXECUTION_ROOT}/links/bazel-"),
        isolated_home: format!("{EXECUTION_ROOT}/home"),
        isolated_tmp: format!("{EXECUTION_ROOT}/tmp"),
        linux_artifact_repository: format!("{EXECUTION_ROOT}/linux-artifacts"),
        namespace_identity: identity.clone(),
        workspace_identity: identity.clone(),
        output_identity: identity.clone(),
        links_identity: identity.clone(),
        home_identity: identity.clone(),
        tmp_identity: identity.clone(),
        linux_artifact_identity: identity,
    }
}

pub fn gates(source_revision: &str) -> Vec<GateEvidence> {
    let launcher = launcher();
    let execution = execution();
    REQUIRED_GATES
        .into_iter()
        .map(|requirement| {
            let tool = match requirement.kind {
                GateKind::Bazel(_) => GateToolEvidence::MacOsBazel {
                    launcher: launcher.clone(),
                },
                GateKind::Linux(profile) => GateToolEvidence::LinuxDocker {
                    docker_path: yolo_release_contracts::DOCKER_SOURCE_LABEL.to_owned(),
                    docker_canonical_path: DOCKER_CANONICAL_TEST_PATH.to_owned(),
                    docker_execution_path: DOCKER_EXECUTION_TEST_PATH.to_owned(),
                    docker_sha256: yolo_release_contracts::DOCKER_CLIENT_SHA256.to_owned(),
                    docker_version: yolo_release_contracts::DOCKER_CLIENT_VERSION.to_owned(),
                    platform: profile.docker_platform.to_owned(),
                    image: profile.image.to_owned(),
                    bazel_path: format!(
                        "{LINUX_BAZEL_EXECUTION_TEST_PATH_PREFIX}{}",
                        profile.docker_architecture
                    ),
                    bazel_sha256: profile.bazel_sha256.to_owned(),
                    bazel_version: yolo_release_contracts::LINUX_BAZEL_VERSION.to_owned(),
                    container_user: LINUX_CONTAINER_TEST_USER.to_owned(),
                    bundle_id: bundles::descriptor(source_revision, profile.architecture)
                        .build_id()
                        .into(),
                },
            };
            GateEvidence {
                gate: requirement.name.to_owned(),
                target: requirement.target.to_owned(),
                source_revision: source_revision.to_owned(),
                tool: tool.clone(),
                execution: execution.clone(),
                commands: requirement.command_evidence(&tool, &execution).unwrap(),
                completed_at_unix_seconds: COMPLETION_SECONDS,
            }
        })
        .collect()
}

pub fn linux_runtimes() -> Vec<LinuxRuntimeEvidence> {
    REQUIRED_GATES
        .into_iter()
        .filter_map(|requirement| match requirement.kind {
            GateKind::Linux(profile) => Some(LinuxRuntimeEvidence {
                image: profile.image.to_owned(),
                bazel_sha256: profile.bazel_sha256.to_owned(),
                descriptor: bundles::descriptor(REVISION, profile.architecture),
                bundle_root: profile.bundle_root.to_owned(),
            }),
            GateKind::Bazel(_) => None,
        })
        .collect()
}

pub fn error_text<T>(result: anyhow::Result<T>) -> String {
    format!("{:#}", result.err().expect("expected error"))
}

pub fn fifo(path: &Path) {
    let path = CString::new(path.as_os_str().as_bytes()).unwrap();
    assert_eq!(
        unsafe { libc::mkfifo(path.as_ptr(), PRIVATE_FILE_MODE as libc::mode_t) },
        SYSTEM_CALL_SUCCESS
    );
}

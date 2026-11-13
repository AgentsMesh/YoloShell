use super::linux_elf_fixture::static_elf;
use super::support::{COMPLETION_SECONDS, REVISION};
use crate::linux_artifact_repository;
use crate::linux_bazel::LinuxBazelTool;
use crate::linux_gate_runner;
use anyhow::{Result, bail};
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use yolo_release_contracts::*;
use yolo_release_gate_system::{
    GateSnapshot, LINUX_AMD64_PROFILE, ProcessExecuting, REQUIRED_GATES,
};
use yolo_release_process_execution::ProcessCommand;
use yolo_release_tool_custody::DockerClientTool;
#[path = "../test_support/container_engine.rs"]
mod engine;

#[test]
fn successful_fixed_container_command_publishes_exact_elf_evidence() {
    let fixture = Fixture::new();
    let snapshot = GateSnapshot::fixture().unwrap();
    linux_artifact_repository::prepare(&snapshot).unwrap();
    let requirement = REQUIRED_GATES[0];
    let (gate, runtime) = linux_gate_runner::run_with_engine(
        requirement,
        REVISION,
        &snapshot,
        &fixture.bazel,
        &fixture.docker,
        &WritingExecutor,
        &engine::TestEngine::default(),
    )
    .unwrap();
    assert_eq!(
        runtime.descriptor.target().architecture.as_str(),
        LINUX_AMD64_RUNTIME_ARCHITECTURE
    );
    assert_eq!(
        runtime.descriptor.target().platform.as_str(),
        LINUX_RUNTIME_PLATFORM
    );
    assert_eq!(gate.completed_at_unix_seconds, COMPLETION_SECONDS);
    assert_eq!(gate.commands.len(), 2);
    assert_eq!(
        gate.commands[0][0],
        fixture.docker.path().display().to_string()
    );
    assert_eq!(
        gate.commands[1][0],
        fixture.docker.path().display().to_string()
    );
    assert_ne!(gate.commands[0][0], DOCKER_SOURCE_LABEL);
}

#[test]
fn failed_container_command_produces_no_gate_or_artifact() {
    let fixture = Fixture::new();
    let snapshot = GateSnapshot::fixture().unwrap();
    let retained = snapshot.root_path();
    linux_artifact_repository::prepare(&snapshot).unwrap();
    assert!(
        linux_gate_runner::run_with_engine(
            REQUIRED_GATES[0],
            REVISION,
            &snapshot,
            &fixture.bazel,
            &fixture.docker,
            &FailingExecutor,
            &engine::TestEngine::default(),
        )
        .is_err()
    );
    drop(snapshot);
    assert!(retained.is_dir());
    // This fake engine created no external resource; discard only its fixture.
    yolo_release_filesystem::restore_owner_write(&retained).unwrap();
    fs::remove_dir_all(retained).unwrap();
}

struct WritingExecutor;

impl ProcessExecuting for WritingExecutor {
    fn execute(&self, command: &ProcessCommand) -> Result<()> {
        let evidence = command.evidence();
        let Some(mount) = evidence
            .iter()
            .find(|value| value.contains("target=/yolo-artifacts"))
        else {
            return Ok(());
        };
        let source = mount
            .strip_prefix("type=bind,source=")
            .unwrap()
            .split(",target=")
            .next()
            .unwrap();
        write_artifact(Path::new(source));
        Ok(())
    }

    fn completed_at_unix_seconds(&self) -> Result<u64> {
        Ok(COMPLETION_SECONDS)
    }
}

struct FailingExecutor;

impl ProcessExecuting for FailingExecutor {
    fn execute(&self, _command: &ProcessCommand) -> Result<()> {
        bail!("forced Docker failure")
    }

    fn completed_at_unix_seconds(&self) -> Result<u64> {
        Ok(COMPLETION_SECONDS)
    }
}

struct FakeBazel(PathBuf);

impl LinuxBazelTool for FakeBazel {
    fn resource_directory(&self) -> Option<&yolo_release_filesystem::DirectoryHandle> {
        None
    }

    fn path(&self) -> &Path {
        &self.0
    }
    fn sha256(&self) -> &str {
        LINUX_AMD64_BAZEL_SHA256
    }
    fn ensure_unchanged(&self) -> Result<()> {
        Ok(())
    }
}

struct FakeDocker(PathBuf);

impl DockerClientTool for FakeDocker {
    fn resource_directory(&self) -> Option<&yolo_release_filesystem::DirectoryHandle> {
        None
    }

    fn path(&self) -> &Path {
        &self.0
    }
    fn canonical_path(&self) -> &Path {
        &self.0
    }
    fn sha256(&self) -> &str {
        DOCKER_CLIENT_SHA256
    }
    fn version(&self) -> &str {
        DOCKER_CLIENT_VERSION
    }
    fn ensure_unchanged(&self) -> Result<()> {
        Ok(())
    }
}

struct Fixture {
    _root: tempfile::TempDir,
    bazel: FakeBazel,
    docker: FakeDocker,
}

impl Fixture {
    fn new() -> Self {
        let root = tempfile::tempdir().unwrap();
        let bazel = root.path().join("bazel");
        let docker = root.path().join("docker");
        fs::write(&bazel, b"bazel").unwrap();
        fs::write(&docker, b"docker").unwrap();
        Self {
            bazel: FakeBazel(bazel),
            docker: FakeDocker(docker),
            _root: root,
        }
    }
}

fn write_artifact(output: &Path) {
    for role in yolo_distribution_bundle::ArtifactRole::ALL {
        let name = role.executable_name();
        let path = output.join(name);
        fs::write(&path, static_elf(LINUX_AMD64_PROFILE.elf_machine)).unwrap();
        fs::set_permissions(&path, fs::Permissions::from_mode(READ_ONLY_EXECUTABLE_MODE)).unwrap();
        fs::write(
            output.join(format!("{name}{LINUX_BUNDLE_VERSION_SUFFIX}")),
            format!("{name} {LINUX_RUNTIME_VERSION}\n"),
        )
        .unwrap();
    }
}

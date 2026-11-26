use super::linux_elf_fixture::static_elf;
use super::support::{REVISION, error_text, fifo};
use super::test_constants::SOURCE_REVISION_JSON_KEY;
use crate::linux_artifact_repository::{
    VERSION_SIZE_ERROR, VERSION_TYPE_ERROR, install, prepare, seal,
};
use std::fs;
use std::io::Read;
use std::os::unix::fs::{PermissionsExt, symlink};
use yolo_distribution_bundle::{ArtifactRole, BundleDescriptor, MANIFEST_FILE_NAME};
use yolo_release_contracts::*;
use yolo_release_gate_system::{
    GateSnapshot, LINUX_AMD64_PROFILE, LINUX_ARM64_PROFILE, LinuxGateProfile,
};

const VERSION_SYMLINK_TARGET_NAME: &str = "version-target";

#[test]
fn exact_verified_elfs_form_a_sealed_bazel_repository_and_manifest() {
    let mut snapshot = GateSnapshot::fixture().unwrap();
    prepare(&snapshot).unwrap();
    let root = tempfile::tempdir().unwrap();
    let amd64 = output(root.path(), LINUX_AMD64_PROFILE);
    let arm64 = output(root.path(), LINUX_ARM64_PROFILE);
    let runtimes = vec![
        install(&snapshot, LINUX_AMD64_PROFILE, &amd64, REVISION).unwrap(),
        install(&snapshot, LINUX_ARM64_PROFILE, &arm64, REVISION).unwrap(),
    ];
    seal(&mut snapshot, REVISION, &runtimes).unwrap();
    snapshot.verify_integrity().unwrap();
    let mut bytes = Vec::new();
    snapshot
        .open_linux_manifest()
        .unwrap()
        .read_to_end(&mut bytes)
        .unwrap();
    let value: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(value[SOURCE_REVISION_JSON_KEY], REVISION);
    assert_eq!(
        value["runtimes"][0]["descriptor"]["target"]["architecture"],
        LINUX_AMD64_RUNTIME_ARCHITECTURE
    );
    assert_eq!(
        value["runtimes"][1]["descriptor"]["target"]["architecture"],
        LINUX_ARM64_RUNTIME_ARCHITECTURE
    );
    assert_eq!(runtimes[0].descriptor.version(), LINUX_RUNTIME_VERSION);
    for runtime in &runtimes {
        let relative = if runtime.descriptor.target().architecture.as_str()
            == LINUX_AMD64_RUNTIME_ARCHITECTURE
        {
            LINUX_AMD64_DOCKER_ARCHITECTURE
        } else {
            LINUX_ARM64_DOCKER_ARCHITECTURE
        };
        let directory = snapshot
            .linux_artifacts()
            .display_path()
            .join(relative);
        let descriptor: BundleDescriptor =
            serde_json::from_slice(&fs::read(directory.join(MANIFEST_FILE_NAME)).unwrap()).unwrap();
        assert_eq!(descriptor, runtime.descriptor);
        assert_eq!(
            fs::read_dir(directory.join(yolo_distribution_bundle::BIN_DIRECTORY))
                .unwrap()
                .count(),
            ArtifactRole::ALL.len()
        );
        assert_eq!(
            descriptor.fields().schema_version,
            yolo_distribution_bundle::BUNDLE_SCHEMA_VERSION
        );
        for role in ArtifactRole::ALL {
            descriptor
                .verify_artifact(
                    role,
                    &fs::read(directory.join(role.relative_path())).unwrap(),
                )
                .unwrap();
        }
        yolo_release_gate_system::audit_bundle_manifest_path(&directory.join(MANIFEST_FILE_NAME))
            .unwrap();
    }
}

#[test]
fn missing_runtime_cannot_be_published_as_a_complete_bundle() {
    let snapshot = GateSnapshot::fixture().unwrap();
    prepare(&snapshot).unwrap();
    let root = tempfile::tempdir().unwrap();
    let directory = output(root.path(), LINUX_AMD64_PROFILE);
    fs::remove_file(directory.join(ArtifactRole::Runtime.executable_name())).unwrap();
    assert!(install(&snapshot, LINUX_AMD64_PROFILE, &directory, REVISION).is_err());
    assert!(
        !snapshot
            .linux_artifacts()
            .display_path()
            .join(LINUX_AMD64_DOCKER_ARCHITECTURE)
            .exists()
    );
}

#[test]
fn version_and_declared_machine_mismatches_are_rejected() {
    let snapshot = GateSnapshot::fixture().unwrap();
    prepare(&snapshot).unwrap();
    let root = tempfile::tempdir().unwrap();
    let output = output(root.path(), LINUX_AMD64_PROFILE);
    fs::write(version_path(&output), b"wrong\n").unwrap();
    assert!(install(&snapshot, LINUX_AMD64_PROFILE, &output, REVISION).is_err());
    fs::write(
        version_path(&output),
        format!("{}\n", yolo_version_output()),
    )
    .unwrap();
    assert!(install(&snapshot, LINUX_ARM64_PROFILE, &output, REVISION).is_err());
}

#[test]
fn version_input_rejects_unbounded_and_indirect_files() {
    let snapshot = GateSnapshot::fixture().unwrap();
    prepare(&snapshot).unwrap();
    let root = tempfile::tempdir().unwrap();
    let output = output(root.path(), LINUX_AMD64_PROFILE);
    let version = version_path(&output);

    fs::remove_file(&version).unwrap();
    fifo(&version);
    let error = error_text(install(&snapshot, LINUX_AMD64_PROFILE, &output, REVISION));
    assert!(error.contains(VERSION_TYPE_ERROR), "{error}");

    fs::remove_file(&version).unwrap();
    let target = root.path().join(VERSION_SYMLINK_TARGET_NAME);
    fs::write(&target, format!("{}\n", yolo_version_output())).unwrap();
    symlink(&target, &version).unwrap();
    assert!(install(&snapshot, LINUX_AMD64_PROFILE, &output, REVISION).is_err());

    fs::remove_file(&version).unwrap();
    let oversized = format!("{}\n{}", yolo_version_output(), yolo_version_output());
    fs::write(&version, oversized).unwrap();
    let error = error_text(install(&snapshot, LINUX_AMD64_PROFILE, &output, REVISION));
    assert!(error.contains(VERSION_SIZE_ERROR), "{error}");
}

fn output(root: &std::path::Path, profile: LinuxGateProfile) -> std::path::PathBuf {
    let output = root.join(profile.docker_architecture);
    fs::create_dir(&output).unwrap();
    for role in yolo_distribution_bundle::ArtifactRole::ALL {
        let name = role.executable_name();
        fs::write(
            output.join(format!("{name}{LINUX_BUNDLE_VERSION_SUFFIX}")),
            format!("{name} {LINUX_RUNTIME_VERSION}\n"),
        )
        .unwrap();
        let path = output.join(name);
        fs::write(&path, static_elf(profile.elf_machine)).unwrap();
        fs::set_permissions(&path, fs::Permissions::from_mode(READ_ONLY_EXECUTABLE_MODE)).unwrap();
    }
    output
}

fn version_path(output: &std::path::Path) -> std::path::PathBuf {
    output.join(format!(
        "{}{LINUX_BUNDLE_VERSION_SUFFIX}",
        yolo_distribution_bundle::ArtifactRole::Runtime.executable_name()
    ))
}

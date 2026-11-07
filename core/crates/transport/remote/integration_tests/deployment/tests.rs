use super::*;
use crate::remote::RemoteError;
use crate::remote::constants::{
    ELF_CLASS_64, ELF_CLASS_OFFSET, ELF_ENDIAN_OFFSET, ELF_HEADER_BYTES, ELF_LITTLE_ENDIAN,
    ELF_MACHINE_OFFSET, ELF_MACHINE_X86_64, ELF_MAGIC, ELF_VERSION_CURRENT, ELF_VERSION_OFFSET,
    MAX_REMOTE_PATH_BYTES, MAX_RUNTIME_BINARY_BYTES, ROOT_PATH, RUNTIME_EXECUTABLE_FILE_NAME,
    SHA256_HEX_LENGTH, TEST_PATH_FILL, VERSION_MAX_BYTES,
};
use yolo_crypto::sha256_hex;
#[path = "../../test_support/runtime_size_constants.rs"]
mod size_constants;

#[test]
fn artifact_rejects_invalid_manifest_hash_version_and_architecture() {
    let bytes = elf(ELF_MACHINE_X86_64);
    assert!(matches!(
        RuntimeArtifact::new(
            "../bad",
            RemoteArchitecture::X86_64,
            bytes.clone(),
            sha256_hex(&bytes)
        ),
        Err(RemoteError::InvalidArtifact(_))
    ));
    let empty = Vec::new();
    assert!(matches!(
        RuntimeArtifact::new(
            "1.0.0",
            RemoteArchitecture::X86_64,
            empty.clone(),
            sha256_hex(&empty)
        ),
        Err(RemoteError::InvalidArtifact(_))
    ));
    assert!(
        RuntimeArtifact::new(
            "x".repeat(VERSION_MAX_BYTES + 1),
            RemoteArchitecture::X86_64,
            bytes.clone(),
            sha256_hex(&bytes)
        )
        .is_err()
    );
    assert!(
        RuntimeArtifact::new(
            "1.0.0",
            RemoteArchitecture::X86_64,
            bytes.clone(),
            "A".repeat(SHA256_HEX_LENGTH)
        )
        .is_err()
    );
    assert!(matches!(
        RuntimeArtifact::new(
            "1.0.0",
            RemoteArchitecture::X86_64,
            bytes.clone(),
            "0".repeat(SHA256_HEX_LENGTH)
        ),
        Err(RemoteError::RuntimeIntegrity(_))
    ));
    assert!(matches!(
        RuntimeArtifact::new("1.0.0", RemoteArchitecture::X86_64, bytes.clone(), "0"),
        Err(RemoteError::RuntimeIntegrity(_))
    ));
    assert!(matches!(
        RuntimeArtifact::new(
            "1.0.0",
            RemoteArchitecture::Aarch64,
            bytes.clone(),
            sha256_hex(&bytes)
        ),
        Err(RemoteError::InvalidArtifact(_))
    ));
}

#[test]
fn artifact_accepts_a_verified_binary_and_remote_architecture_aliases() {
    let bytes = elf(ELF_MACHINE_X86_64);
    let artifact = RuntimeArtifact::new(
        "1.0.0",
        RemoteArchitecture::X86_64,
        bytes.clone(),
        sha256_hex(&bytes),
    )
    .unwrap();
    assert_eq!(artifact.version(), "1.0.0");
    assert_eq!(artifact.platform(), RemotePlatform::Linux);
    assert_eq!(artifact.bytes(), bytes);
    assert_eq!(artifact.sha256(), sha256_hex(artifact.bytes()));
    assert!(artifact.architecture().matches_remote("amd64"));
    assert!(!artifact.architecture().matches_remote("arm64"));
}

#[test]
fn artifact_size_and_plan_nonce_boundaries_are_explicit() {
    use size_constants::*;
    assert_eq!(
        MAX_RUNTIME_BINARY_BYTES as u64,
        yolo_distribution_bundle::MAX_ARTIFACT_BYTES
    );
    assert!(!super::artifact::valid_binary_size(0));
    for bytes in [
        RETIRED_RUNTIME_LIMIT + SIZE_BOUNDARY_STEP,
        UNIFIED_COVERAGE_IMAGE_BYTES,
        MAX_RUNTIME_BINARY_BYTES,
    ] {
        assert!(super::artifact::valid_binary_size(bytes));
    }
    assert!(!super::artifact::valid_binary_size(
        MAX_RUNTIME_BINARY_BYTES + 1
    ));
    let bytes = elf(ELF_MACHINE_X86_64);
    let artifact = RuntimeArtifact::new(
        "1.0.0",
        RemoteArchitecture::X86_64,
        bytes.clone(),
        sha256_hex(&bytes),
    )
    .unwrap();
    let runtime = crate::remote::RemoteRuntimeSpec::new("/opt/yolo/yolo", "/var/lib/yolo").unwrap();
    let first = super::plan::DeploymentPlan::new(&runtime, &artifact).unwrap();
    let second = super::plan::DeploymentPlan::new(&runtime, &artifact).unwrap();
    let first_stage = first.stage(&artifact);
    let second_stage = second.stage(&artifact);
    let first_activation = first.activation(&artifact);
    let second_activation = second.activation(&artifact);
    assert_ne!(first_stage.temporary_path, second_stage.temporary_path);
    assert_ne!(
        first_activation.temporary_link,
        second_activation.temporary_link
    );
    assert!(first.release_path().contains("linux-x86_64"));
    let wrong_name =
        crate::remote::RemoteRuntimeSpec::new("/opt/yolo/not-host", "/var/lib/yolo").unwrap();
    assert!(super::plan::DeploymentPlan::new(&wrong_name, &artifact).is_err());
    let root_parent = crate::remote::RemoteRuntimeSpec::new("/yolo", "/var/lib/yolo").unwrap();
    assert!(super::plan::DeploymentPlan::new(&root_parent, &artifact).is_err());
    let suffix = format!("{ROOT_PATH}{RUNTIME_EXECUTABLE_FILE_NAME}");
    let parent_bytes = MAX_REMOTE_PATH_BYTES - ROOT_PATH.len() - suffix.len();
    let executable = format!("{ROOT_PATH}{}{suffix}", TEST_PATH_FILL.repeat(parent_bytes));
    assert_eq!(executable.len(), MAX_REMOTE_PATH_BYTES);
    let bounded = crate::remote::RemoteRuntimeSpec::new(executable, "/state").unwrap();
    assert!(super::plan::DeploymentPlan::new(&bounded, &artifact).is_err());
}

fn elf(machine: u16) -> Vec<u8> {
    let mut bytes = vec![0; ELF_HEADER_BYTES];
    bytes[..ELF_MAGIC.len()].copy_from_slice(ELF_MAGIC);
    bytes[ELF_CLASS_OFFSET] = ELF_CLASS_64;
    bytes[ELF_ENDIAN_OFFSET] = ELF_LITTLE_ENDIAN;
    bytes[ELF_VERSION_OFFSET] = ELF_VERSION_CURRENT;
    bytes[ELF_MACHINE_OFFSET..ELF_HEADER_BYTES].copy_from_slice(&machine.to_le_bytes());
    bytes
}

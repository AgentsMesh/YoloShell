use yolo_distribution_bundle::{
    Architecture, ArtifactDescriptor, ArtifactRole, BundleArtifacts, BundleDescriptor, Platform,
    Target,
};
use yolo_release_contracts::*;
use yolo_release_foundation::digest::bytes_sha256;
use yolo_remote::{RemoteArchitecture, RemotePlatform, RuntimeArtifact};

const ELF_VERSION_OFFSET: usize = 6;
const ELF_CURRENT_VERSION: u8 = 1;

#[test]
fn manifest_targets_construct_real_remote_runtime_artifacts() {
    for (profile, architecture) in [
        (
            crate::requirements::LINUX_AMD64_PROFILE,
            RemoteArchitecture::X86_64,
        ),
        (
            crate::requirements::LINUX_ARM64_PROFILE,
            RemoteArchitecture::Aarch64,
        ),
    ] {
        let bytes = elf(profile.elf_machine);
        let artifact = RuntimeArtifact::new(
            LINUX_RUNTIME_VERSION,
            architecture,
            bytes.clone(),
            bytes_sha256(&bytes),
        )
        .unwrap();
        assert_eq!(artifact.version(), LINUX_RUNTIME_VERSION);
        assert_eq!(artifact.architecture(), architecture);
        assert_eq!(artifact.platform(), RemotePlatform::Linux);
        let component = ArtifactDescriptor::new(artifact.sha256(), bytes.len() as u64).unwrap();
        let descriptor = BundleDescriptor::new(
            yolo_release_gate_test_support::support::REVISION,
            LINUX_RUNTIME_VERSION,
            Target {
                platform: Platform::Linux,
                architecture: match architecture {
                    RemoteArchitecture::X86_64 => Architecture::X86_64,
                    RemoteArchitecture::Aarch64 => Architecture::Aarch64,
                },
            },
            BundleArtifacts { runtime: component },
        )
        .unwrap();
        for role in ArtifactRole::ALL {
            descriptor.verify_artifact(role, &bytes).unwrap();
        }
    }
}

fn elf(machine: u16) -> Vec<u8> {
    let mut bytes = vec![0; ELF_HEADER_BYTES];
    bytes[..ELF_MAGIC.len()].copy_from_slice(ELF_MAGIC);
    bytes[ELF_CLASS_OFFSET] = ELF_64_CLASS;
    bytes[ELF_DATA_OFFSET] = ELF_LITTLE_ENDIAN_DATA;
    bytes[ELF_VERSION_OFFSET] = ELF_CURRENT_VERSION;
    bytes[ELF_MACHINE_OFFSET..ELF_HEADER_BYTES].copy_from_slice(&machine.to_le_bytes());
    bytes
}

use super::constants::*;
use super::fixture::{MemoryConnection, connect_memory_with_fs};
use super::memory_fs::MemoryFilesystem;
use crate::remote::constants::{
    ELF_CLASS_64, ELF_CLASS_OFFSET, ELF_ENDIAN_OFFSET, ELF_HEADER_BYTES, ELF_LITTLE_ENDIAN,
    ELF_MACHINE_AARCH64, ELF_MACHINE_OFFSET, ELF_MACHINE_X86_64, ELF_MAGIC, ELF_VERSION_CURRENT,
    ELF_VERSION_OFFSET,
};
use crate::remote::{RemoteArchitecture, RemoteRuntimeSpec, RuntimeArtifact};
use yolo_crypto::sha256_hex;

pub async fn deployment_fixture(version: &str) -> MemoryConnection {
    let filesystem = MemoryFilesystem::default();
    filesystem.set_version(version);
    connect_memory_with_fs(false, filesystem).await.unwrap()
}

pub async fn wait_for_temporary_entries(filesystem: &MemoryFilesystem, expected: usize) {
    let settled = tokio::time::timeout(SFTP_LIFECYCLE_TIMEOUT, async {
        while filesystem.temporary_entries() != expected {
            tokio::task::yield_now().await;
        }
    })
    .await;
    assert!(settled.is_ok());
}

pub fn runtime() -> RemoteRuntimeSpec {
    RemoteRuntimeSpec::new(MEMORY_RUNTIME, MEMORY_HOME).unwrap()
}

pub fn artifact(version: &str, fill: u8) -> RuntimeArtifact {
    artifact_with_size(version, fill, TEST_BINARY_BYTES)
}

pub fn artifact_for(architecture: RemoteArchitecture, version: &str, fill: u8) -> RuntimeArtifact {
    artifact_for_size(architecture, version, fill, TEST_BINARY_BYTES)
}

pub fn artifact_with_size(version: &str, fill: u8, size: usize) -> RuntimeArtifact {
    artifact_for_size(RemoteArchitecture::X86_64, version, fill, size)
}

fn artifact_for_size(
    architecture: RemoteArchitecture,
    version: &str,
    fill: u8,
    size: usize,
) -> RuntimeArtifact {
    let machine = match architecture {
        RemoteArchitecture::Aarch64 => ELF_MACHINE_AARCH64,
        RemoteArchitecture::X86_64 => ELF_MACHINE_X86_64,
    };
    let mut bytes = vec![fill; size.max(ELF_HEADER_BYTES)];
    bytes[..ELF_MAGIC.len()].copy_from_slice(ELF_MAGIC);
    bytes[ELF_CLASS_OFFSET] = ELF_CLASS_64;
    bytes[ELF_ENDIAN_OFFSET] = ELF_LITTLE_ENDIAN;
    bytes[ELF_VERSION_OFFSET] = ELF_VERSION_CURRENT;
    bytes[ELF_MACHINE_OFFSET..ELF_HEADER_BYTES].copy_from_slice(&machine.to_le_bytes());
    let digest = sha256_hex(&bytes);
    RuntimeArtifact::new(version, architecture, bytes, digest).unwrap()
}

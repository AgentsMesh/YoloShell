use super::constants::*;
use std::{
    fs::{self, OpenOptions},
    io::Write,
    os::unix::fs::{OpenOptionsExt, PermissionsExt},
    path::{Path, PathBuf},
};
use uuid::Uuid;
use yolo_distribution_bundle::*;
use yolo_distribution_release_store::{EXECUTABLE_MODE, MANIFEST_MODE, ReleaseStore};

pub struct Fixture {
    pub temporary: tempfile::TempDir,
    pub root: PathBuf,
}

impl Fixture {
    pub fn new() -> Self {
        let temporary = tempfile::tempdir().unwrap();
        let root = temporary
            .path()
            .canonicalize()
            .unwrap()
            .join(INSTALL_DIRECTORY);
        ReleaseStore::open(&root).unwrap();
        Self { temporary, root }
    }
    pub fn store(&self) -> ReleaseStore {
        ReleaseStore::open(&self.root).unwrap()
    }
    pub fn stage(&self, fill: u8) -> (Uuid, BundleDescriptor) {
        let payloads = ArtifactRole::ALL.map(|role| payload(role, fill));
        self.stage_payloads(payloads)
    }
    pub fn stage_payloads(
        &self,
        payloads: [Vec<u8>; ArtifactRole::ALL.len()],
    ) -> (Uuid, BundleDescriptor) {
        let descriptor = descriptor(&payloads);
        let stage = Uuid::new_v4();
        let path = self.store().stage_path(stage).unwrap();
        directory(&path);
        directory(&path.join(BIN_DIRECTORY));
        for (role, bytes) in ArtifactRole::ALL.into_iter().zip(&payloads) {
            write(&path.join(role.relative_path()), bytes, EXECUTABLE_MODE);
        }
        write(
            &path.join(MANIFEST_FILE_NAME),
            &serde_json::to_vec(&descriptor).unwrap(),
            MANIFEST_MODE,
        );
        (stage, descriptor)
    }
}

pub fn directory(path: &Path) {
    fs::create_dir(path).unwrap();
    fs::set_permissions(path, fs::Permissions::from_mode(PRIVATE_DIRECTORY_MODE)).unwrap();
}

pub fn write(path: &Path, bytes: &[u8], mode: u32) {
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(mode)
        .open(path)
        .unwrap();
    file.write_all(bytes).unwrap();
}

pub fn payload(role: ArtifactRole, fill: u8) -> Vec<u8> {
    let mut bytes = vec![0; ELF_BYTES];
    bytes[..ELF_MAGIC.len()].copy_from_slice(ELF_MAGIC);
    bytes[ELF_CLASS_OFFSET] = ELF_64;
    bytes[ELF_ENDIAN_OFFSET] = ELF_LITTLE_ENDIAN;
    bytes[ELF_VERSION_OFFSET] = ELF_CURRENT;
    bytes[ELF_MACHINE_OFFSET..].copy_from_slice(&ELF_X86_64.to_le_bytes());
    bytes.extend_from_slice(role.as_str().as_bytes());
    bytes.push(fill);
    bytes
}

pub fn descriptor(payloads: &[Vec<u8>; ArtifactRole::ALL.len()]) -> BundleDescriptor {
    let [runtime] = payloads.each_ref().map(|bytes| {
        ArtifactDescriptor::new(yolo_crypto::sha256_hex(bytes), bytes.len() as u64).unwrap()
    });
    BundleDescriptor::new(
        REVISION,
        VERSION,
        Target {
            platform: Platform::Linux,
            architecture: Architecture::X86_64,
        },
        BundleArtifacts { runtime },
    )
    .unwrap()
}

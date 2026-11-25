use crate::{constants::*, file_seal::FileSeal};
use anyhow::{Result, ensure};
use std::{
    ffi::CString,
    fs::File,
    os::{fd::FromRawFd, unix::fs::FileExt},
};
use yolo_crypto::Sha256Hasher;
use yolo_distribution_bundle::{ArtifactDescriptor, Target, binary_target};
use yolo_paths::directory::PrivateDirectory;

pub(crate) struct HeldFile {
    file: File,
    name: String,
    mode: u32,
    maximum: u64,
    seal: FileSeal,
}

impl HeldFile {
    pub(crate) fn open(
        directory: &PrivateDirectory,
        name: &str,
        mode: u32,
        maximum: u64,
    ) -> Result<Self> {
        directory.verify_path()?;
        let file = open_at(directory, name)?;
        let seal = FileSeal::capture(&file, &directory.path().join(name), mode, maximum)?;
        let value = Self {
            file,
            name: name.into(),
            mode,
            maximum,
            seal,
        };
        value.verify(directory)?;
        Ok(value)
    }

    pub(crate) fn verify(&self, directory: &PrivateDirectory) -> Result<()> {
        directory.verify_path()?;
        let path = directory.path().join(&self.name);
        ensure!(
            FileSeal::capture(&self.file, &path, self.mode, self.maximum)? == self.seal,
            IDENTITY_CHANGED
        );
        let reopened = open_at(directory, &self.name)?;
        ensure!(
            FileSeal::capture(&reopened, &path, self.mode, self.maximum)? == self.seal,
            IDENTITY_CHANGED
        );
        directory.verify_path()
    }

    pub(crate) fn read_manifest(&self, directory: &PrivateDirectory) -> Result<Vec<u8>> {
        self.verify(directory)?;
        let mut bytes = vec![0; usize::try_from(self.seal.length)?];
        self.file.read_exact_at(&mut bytes, 0)?;
        self.verify(directory)?;
        Ok(bytes)
    }

    pub(crate) fn verify_artifact(
        &self,
        directory: &PrivateDirectory,
        artifact: &ArtifactDescriptor,
        target: Target,
    ) -> Result<()> {
        ensure!(self.seal.length == artifact.size_bytes(), FILE_INTEGRITY);
        let mut digest = Sha256Hasher::new();
        let mut buffer = [0; HASH_CHUNK_BYTES];
        let mut header = Vec::with_capacity(HEADER_PROBE_BYTES);
        let mut offset = 0_u64;
        while offset <= artifact.size_bytes() {
            let remaining = artifact
                .size_bytes()
                .saturating_sub(offset)
                .saturating_add(1);
            let limit = buffer.len().min(usize::try_from(remaining)?);
            let read = self.file.read_at(&mut buffer[..limit], offset)?;
            if read == 0 {
                break;
            }
            let retained = read.min(HEADER_PROBE_BYTES.saturating_sub(header.len()));
            header.extend_from_slice(&buffer[..retained]);
            digest.update(&buffer[..read]);
            offset += read as u64;
        }
        ensure!(
            offset == artifact.size_bytes() && digest.finish_hex() == artifact.sha256(),
            FILE_INTEGRITY
        );
        ensure!(binary_target(&header) == Some(target), TARGET_MISMATCH);
        self.verify(directory)
    }

    pub(crate) fn sync(&self) -> Result<()> {
        self.file.sync_all().map_err(Into::into)
    }
}

fn open_at(directory: &PrivateDirectory, name: &str) -> Result<File> {
    let name = CString::new(name)?;
    let flags =
        libc::O_RDONLY | libc::O_NOFOLLOW | libc::O_CLOEXEC | libc::O_NONBLOCK | libc::O_NOCTTY;
    let descriptor = unsafe { libc::openat(directory.descriptor(), name.as_ptr(), flags) };
    if descriptor < 0 {
        return Err(std::io::Error::last_os_error().into());
    }
    Ok(unsafe { File::from_raw_fd(descriptor) })
}

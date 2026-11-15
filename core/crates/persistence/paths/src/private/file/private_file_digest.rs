use super::{FileIdentity, TemporaryPrivateFile};
use anyhow::{Context, Result};
use sha2::{Digest, Sha256};
use std::fs::File;
use std::os::unix::fs::FileExt;
use std::path::Path;

const DIGEST_CHUNK_BYTES: usize = 64 * 1024;
const DIGEST_CHUNK_BYTES_U64: u64 = DIGEST_CHUNK_BYTES as u64;
const FILE_START_OFFSET: u64 = 0;

#[derive(Debug, PartialEq, Eq)]
pub(super) struct PrivateFileDigest(Vec<u8>);

impl PrivateFileDigest {
    pub(super) fn capture(file: &File, length: u64) -> Result<Self> {
        let mut digest = Sha256::new();
        let mut buffer = vec![0_u8; DIGEST_CHUNK_BYTES];
        let mut offset = FILE_START_OFFSET;
        while offset < length {
            let remaining = length - offset;
            let count = remaining.min(DIGEST_CHUNK_BYTES_U64) as usize;
            file.read_exact_at(&mut buffer[..count], offset)
                .context("read private file content digest")?;
            digest.update(&buffer[..count]);
            offset += count as u64;
        }
        Ok(Self(digest.finalize().to_vec()))
    }

    fn verify(
        &self,
        file: &File,
        expected: FileIdentity,
        path: &Path,
        max_bytes: u64,
    ) -> Result<FileIdentity> {
        let before = FileIdentity::inspect_file_bounded(file, path, max_bytes)?;
        anyhow::ensure!(
            before.same_content_version(expected),
            "private file content version changed"
        );
        let current = Self::capture(file, before.length())?;
        let after = FileIdentity::inspect_file_bounded(file, path, max_bytes)?;
        anyhow::ensure!(
            before.unchanged(after),
            "private file changed during digest check"
        );
        anyhow::ensure!(&current == self, "private file content digest changed");
        Ok(after)
    }
}

impl TemporaryPrivateFile {
    pub fn sync_and_inspect(&mut self, path: &Path, max_bytes: u64) -> Result<()> {
        self.file
            .sync_all()
            .context("sync temporary private file")?;
        self.refresh_digest(path, max_bytes)
    }

    pub(super) fn refresh_digest(&mut self, path: &Path, max_bytes: u64) -> Result<()> {
        self.identity = FileIdentity::inspect_file_bounded(&self.file, path, max_bytes)?;
        self.digest = PrivateFileDigest::capture(&self.file, self.identity.length())?;
        self.identity
            .verify_file_bounded(&self.file, self.identity.length(), path, max_bytes)
    }

    pub(super) fn verify_digest_after_move(
        &self,
        path: &Path,
        max_bytes: u64,
    ) -> Result<FileIdentity> {
        self.digest
            .verify(&self.file, self.identity, path, max_bytes)
    }

    pub(super) fn verify_previous_digest_after_move(
        &self,
        path: &Path,
        max_bytes: u64,
    ) -> Result<Option<FileIdentity>> {
        self.previous
            .as_ref()
            .zip(self.previous_digest.as_ref())
            .map(|(opened, digest)| digest.verify(&opened.file, opened.identity, path, max_bytes))
            .transpose()
    }
}

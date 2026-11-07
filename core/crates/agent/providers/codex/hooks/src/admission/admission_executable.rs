use anyhow::{Context, Result, ensure};
use ring::digest::{Context as DigestContext, SHA256};
use serde::{Deserialize, Serialize};
use std::fs::{File, OpenOptions};
use std::os::unix::ffi::OsStrExt;
use std::os::unix::fs::{FileExt, OpenOptionsExt};
use std::path::{Path, PathBuf};

#[path = "admission_executable_identity.rs"]
mod identity_record;
#[path = "admission_executable_kind.rs"]
mod kind;
pub use identity_record::CodexExecutableIdentity;
use identity_record::capture as identity;
pub use kind::CodexExecutableKind;

include!("admission_executable_mapping.rs");

const CONTENT_DIGEST_DOMAIN: &str = "yolo.codex-admission.executable.v1";
const DIGEST_CHUNK_BYTES: usize = 64 * 1_024;
const EXECUTABLE_OPEN_FLAGS: i32 =
    libc::O_NOFOLLOW | libc::O_CLOEXEC | libc::O_NONBLOCK | libc::O_NOCTTY;
const FILE_START_OFFSET: u64 = 0;
const RAW_SHA256_BYTES: usize = ring::digest::SHA256_OUTPUT_LEN;
const EXECUTABLE_CHANGED: &str = "Codex admission executable changed while captured";
const EXECUTABLE_INVALID: &str = "Codex admission executable is invalid";

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CodexExecutableAttestation {
    path: PathBuf,
    identity: CodexExecutableIdentity,
    #[serde(with = "crate::sha256_digest::compact")]
    content_digest: crate::sha256_digest::Sha256Digest,
}

impl CodexExecutableAttestation {
    pub(crate) fn capture(path: &Path, kind: CodexExecutableKind) -> Result<Self> {
        Self::capture_open(path, kind).map(|(attestation, _)| attestation)
    }

    fn capture_open(path: &Path, kind: CodexExecutableKind) -> Result<(Self, File)> {
        validate_capture_path(path)?;
        let named = identity(&std::fs::symlink_metadata(path)?, kind)?;
        let file = OpenOptions::new()
            .read(true)
            .custom_flags(EXECUTABLE_OPEN_FLAGS)
            .open(path)
            .context(EXECUTABLE_INVALID)?;
        let opened = identity(&file.metadata()?, kind)?;
        ensure!(named == opened, EXECUTABLE_CHANGED);
        let raw_digest = content_digest(&file, opened.length())?;
        let content_digest =
            crate::sha256_digest::Sha256Digest::capture(&(CONTENT_DIGEST_DOMAIN, raw_digest))?;
        let after = identity(&file.metadata()?, kind)?;
        let renamed = identity(&std::fs::symlink_metadata(path)?, kind)?;
        ensure!(opened == after && after == renamed, EXECUTABLE_CHANGED);
        Ok((
            Self {
                path: path.to_path_buf(),
                identity: opened,
                content_digest,
            },
            file,
        ))
    }

    pub(crate) fn validate_for(&self, path: &Path, kind: CodexExecutableKind) -> Result<()> {
        ensure!(self.path == path, EXECUTABLE_INVALID);
        validate_bound_path(path)?;
        self.identity.validate(kind)
    }

    pub fn verify_current(&self, kind: CodexExecutableKind) -> Result<()> {
        ensure!(
            Self::capture(&self.path, kind)? == *self,
            EXECUTABLE_CHANGED
        );
        Ok(())
    }
    pub fn path(&self) -> &Path {
        &self.path
    }
    pub(crate) fn identity(&self) -> CodexExecutableIdentity {
        self.identity
    }
    pub(crate) fn content_digest(&self) -> crate::sha256_digest::Sha256Digest {
        self.content_digest
    }
}

fn validate_bound_path(path: &Path) -> Result<()> {
    crate::paths::validation::validate_absolute(path)?;
    ensure!(
        path.as_os_str().as_bytes().len() <= crate::constants::MAX_RUNTIME_PATH_BYTES,
        EXECUTABLE_INVALID
    );
    Ok(())
}

fn validate_capture_path(path: &Path) -> Result<()> {
    validate_bound_path(path)?;
    ensure!(std::fs::canonicalize(path)? == path, EXECUTABLE_INVALID);
    Ok(())
}

fn content_digest(file: &File, length: u64) -> Result<[u8; RAW_SHA256_BYTES]> {
    let mut digest = DigestContext::new(&SHA256);
    let mut buffer = vec![u8::default(); DIGEST_CHUNK_BYTES];
    let mut offset = FILE_START_OFFSET;
    while offset < length {
        let count = usize::try_from(length - offset)
            .unwrap_or(usize::MAX)
            .min(buffer.len());
        let read = file.read_at(&mut buffer[..count], offset)?;
        ensure!(read != usize::default(), EXECUTABLE_CHANGED);
        digest.update(&buffer[..read]);
        offset = offset
            .checked_add(u64::try_from(read)?)
            .context(EXECUTABLE_CHANGED)?;
    }
    Ok(digest.finish().as_ref().try_into()?)
}
